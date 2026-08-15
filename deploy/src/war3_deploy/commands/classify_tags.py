"""Create resumable, AI-assisted gameplay / franchise / theme tag candidates."""

from __future__ import annotations

import argparse
import json
import os
import re
import urllib.error
import urllib.request
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from urllib.parse import urlparse

from ..catalog import emit_report, load_catalog, save_catalog, write_atomic
from ..progress import Progress
from ..tagging import (
    ALLOWED_TAGS,
    TAG_SCHEMA_VERSION,
    canonicalize_tag,
    map_text,
    seed_tags,
    taxonomy_prompt,
)

DEFAULT_MODEL = "mlx-community/Qwen3-4B-Instruct-2507-4bit"
DEFAULT_ENDPOINT = "http://127.0.0.1:8080/v1/chat/completions"

# Remote providers (e.g. OpenRouter) need a bearer token.  Set it in .env as
# OPENROUTER_API_KEY, export it, or pass --api-key.  A missing key is fine for
# local endpoints like llama.cpp / MLX that do not authenticate.
API_KEY_ENV = "OPENROUTER_API_KEY"


class TruncatedModelReply(RuntimeError):
    """The provider used all completion tokens before it returned JSON."""


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/maps.json")
    parser.add_argument("--endpoint", default=os.getenv("WAR3_TAG_LLM_ENDPOINT", DEFAULT_ENDPOINT))
    parser.add_argument("--model", default=os.getenv("WAR3_TAG_LLM_MODEL", DEFAULT_MODEL))
    parser.add_argument("--api-key", default=os.getenv(API_KEY_ENV), help=f"Bearer token for remote endpoints (env: {API_KEY_ENV})")
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument(
        "--workers",
        type=int,
        default=1,
        help="Concurrent model requests; candidate writes remain serialized (default: 1)",
    )
    parser.add_argument("--output", type=Path, help="Defaults to catalog/tag-candidates.jsonl")
    parser.add_argument("--limit", type=int, help="Classify only the first N records (smoke testing)")
    parser.add_argument(
        "--no-new-tags",
        action="store_true",
        help="Use a closed taxonomy; by default the model may propose controlled namespace extensions",
    )
    parser.add_argument(
        "--reasoning",
        choices=("off", "omit"),
        default="off",
        help="Send OpenRouter reasoning.enabled=false (default) or omit the provider-specific setting",
    )
    parser.add_argument("--apply", action="store_true", help="Write completed candidates into maps.json")
    parser.add_argument("--dry-run", action="store_true", help="Show deterministic seeds without calling AI")


def candidate_path(root: Path, output: Path | None) -> Path:
    return output.resolve() if output else root / "catalog" / "tag-candidates.jsonl"


def load_completed(path: Path) -> dict[str, dict]:
    completed: dict[str, dict] = {}
    if not path.is_file():
        return completed
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            if line.strip():
                item = json.loads(line)
                digest = str(item.get("sha256", ""))
                if digest:
                    completed[digest] = item
    return completed


def normalize_candidate(candidate: dict) -> dict:
    """Repair namespace slips in checkpoints without sending another request."""
    tags = list(
        dict.fromkeys(tag for raw_tag in candidate.get("tags", []) if (tag := canonicalize_tag(raw_tag)))
    )
    evidence: list[object] = []
    for item in candidate.get("tag_evidence", []):
        if not isinstance(item, str) or not item.startswith("ai:taxonomy_extension:"):
            evidence.append(item)
            continue
        tag = canonicalize_tag(item.removeprefix("ai:taxonomy_extension:"))
        if tag and tag not in ALLOWED_TAGS:
            evidence.append(f"ai:taxonomy_extension:{tag}")
    if tags == candidate.get("tags", []) and evidence == candidate.get("tag_evidence", []):
        return candidate
    return {**candidate, "tags": tags, "tag_evidence": evidence}


def parse_json_reply(content: object) -> dict:
    """Accept JSON, or the harmless Markdown fence some providers add."""
    if not isinstance(content, str):
        raise json.JSONDecodeError("completion content is not text", "", 0)
    text = content.strip()
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        fenced = re.fullmatch(r"```(?:json)?\s*(\{.*\})\s*```", text, flags=re.DOTALL | re.IGNORECASE)
        if not fenced:
            raise
        value = json.loads(fenced.group(1))
    if not isinstance(value, dict):
        raise json.JSONDecodeError("completion is not an object", text, 0)
    return value


def request_tags(
    endpoint: str,
    model: str,
    batch: list[dict],
    api_key: str | None = None,
    *,
    allow_new_tags: bool = True,
    disable_reasoning: bool = True,
) -> list[dict]:
    try:
        return _request_tags_once(
            endpoint,
            model,
            batch,
            api_key=api_key,
            allow_new_tags=allow_new_tags,
            disable_reasoning=disable_reasoning,
        )
    except TruncatedModelReply:
        # A provider can still ignore the reasoning flag or exceed the compact
        # JSON budget. Retry smaller independent requests instead of dropping
        # the whole batch; the original batch-local i values are preserved.
        if len(batch) == 1:
            raise SystemExit(
                "tag model truncated a one-map response; it must return the compact JSON only"
            ) from None
        middle = len(batch) // 2
        return request_tags(
            endpoint,
            model,
            batch[:middle],
            api_key,
            allow_new_tags=allow_new_tags,
            disable_reasoning=disable_reasoning,
        ) + request_tags(
            endpoint,
            model,
            batch[middle:],
            api_key,
            allow_new_tags=allow_new_tags,
            disable_reasoning=disable_reasoning,
        )


def _request_tags_once(
    endpoint: str,
    model: str,
    batch: list[dict],
    api_key: str | None = None,
    *,
    allow_new_tags: bool = True,
    disable_reasoning: bool = True,
) -> list[dict]:
    payload = {
        "model": model,
        "temperature": 0,
        # The reply uses compact batch-local indexes instead of 64-char SHA
        # strings.  This leaves enough room for a whole batch while bounding
        # a malformed answer before it turns into a multi-minute generation.
        "max_tokens": min(2048, 64 + 32 * len(batch)),
        "messages": [
            {"role": "system", "content": taxonomy_prompt(allow_new_tags)},
            {"role": "user", "content": json.dumps({"maps": batch}, ensure_ascii=False)},
        ],
    }
    # OpenRouter normalises this for models that support hidden/visible
    # reasoning.  It prevents a reasoning-capable Flash model from spending
    # the JSON response budget on chain-of-thought before the first record.
    if disable_reasoning and urlparse(endpoint).hostname == "openrouter.ai":
        payload["reasoning"] = {"enabled": False}
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    request = urllib.request.Request(
        endpoint,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers=headers,
    )
    try:
        with urllib.request.urlopen(request, timeout=300) as response:
            reply = json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError as error:
        raise SystemExit(f"tag model unavailable at {endpoint}: {error}") from error
    try:
        choice = reply["choices"][0]
        finish_reason = choice.get("finish_reason")
        if finish_reason == "length":
            raise TruncatedModelReply
        if finish_reason not in (None, "stop"):
            raise SystemExit(
                f"tag model stopped with finish_reason={finish_reason!r}; "
                "reduce --batch-size or keep --reasoning off"
            )
        value = parse_json_reply(choice["message"]["content"])
        maps = value["maps"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"tag model returned invalid JSON: {reply!r}") from error
    return maps if isinstance(maps, list) else []


def validated(record: dict, response: dict | None, *, allow_new_tags: bool = True) -> dict:
    seeds, evidence = seed_tags(record)
    raw_tags = (response or {}).get("tags", [])
    normal_tags = [canonicalize_tag(tag) for tag in raw_tags]
    allowed = [tag for tag in normal_tags if tag in ALLOWED_TAGS]
    extensions: list[str] = []
    if allow_new_tags:
        for tag in normal_tags:
            if tag and tag not in ALLOWED_TAGS and tag not in extensions:
                extensions.append(tag)
            if len(extensions) == 3:
                break
    allowed.extend(extensions)
    tags = list(dict.fromkeys(allowed))
    if not any(tag.startswith("玩法:") for tag in tags):
        tags.extend(tag for tag in seeds if tag.startswith("玩法:"))
    if not any(tag.startswith("玩法:") for tag in tags):
        tags.append("玩法:其他")
    confidence = str((response or {}).get("confidence", "low"))
    if confidence not in {"high", "medium", "low"}:
        confidence = "low"
    if response is not None:
        evidence.append("ai:taxonomy_classification")
        evidence.extend(f"ai:taxonomy_extension:{tag}" for tag in extensions)
    return {
        "sha256": record["sha256"],
        "tags": tags,
        "tag_confidence": confidence,
        "tag_evidence": evidence,
        "tag_schema_version": TAG_SCHEMA_VERSION,
    }


def write_candidates(path: Path, candidates: dict[str, dict]) -> None:
    text = "".join(
        json.dumps(candidates[digest], ensure_ascii=False, separators=(",", ":")) + "\n"
        for digest in sorted(candidates)
    )
    write_atomic(path, text)


def model_input(record: dict, index: int) -> dict:
    title, description = map_text(record)
    seeds, _ = seed_tags(record)
    return {
        "i": index,
        "title": title,
        "description": description[:600],
        "legacy_collection_hint": record.get("collection"),
        "candidate_tags": seeds,
    }


def run(args: argparse.Namespace) -> None:
    if args.batch_size < 1:
        raise SystemExit("--batch-size must be positive")
    if args.workers < 1:
        raise SystemExit("--workers must be positive")
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    all_maps = catalog["maps"]
    if args.limit is not None and args.limit < 1:
        raise SystemExit("--limit must be positive")
    maps = all_maps[: args.limit]
    if args.apply and len(maps) != len(all_maps):
        raise SystemExit("--apply cannot be used with --limit")
    output = candidate_path(root, args.output)
    loaded_candidates = load_completed(output)
    candidates = {
        digest: normalize_candidate(candidate) for digest, candidate in loaded_candidates.items()
    }
    checkpoint_repaired = candidates != loaded_candidates
    known = {str(record.get("sha256", "")) for record in maps}
    # A changed taxonomy policy is a changed classification pass.  Never let
    # a pre-v2 closed-taxonomy candidate silently bypass the current model.
    candidates = {
        digest: value
        for digest, value in candidates.items()
        if digest in known and value.get("tag_schema_version") == TAG_SCHEMA_VERSION
    }
    if checkpoint_repaired:
        write_candidates(output, candidates)

    pending = [record for record in maps if record["sha256"] not in candidates]
    batches = [pending[start : start + args.batch_size] for start in range(0, len(pending), args.batch_size)]

    def classify_batch(records: list[dict]) -> tuple[list[dict], dict[int, dict]]:
        if args.dry_run:
            return records, {}
        response = request_tags(
            args.endpoint,
            args.model,
            [model_input(record, index) for index, record in enumerate(records)],
            api_key=args.api_key,
            allow_new_tags=not args.no_new_tags,
            disable_reasoning=args.reasoning == "off",
        )
        answers = {
            int(item["i"]): item
            for item in response
            if isinstance(item, dict) and isinstance(item.get("i"), int)
        }
        return records, answers

    progress = Progress(len(pending), "maps")
    # Requests may run concurrently, but only this thread mutates and writes
    # the checkpoint.  An interruption therefore leaves a valid JSONL file
    # containing every fully completed batch.
    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(classify_batch, records) for records in batches]
        for future in as_completed(futures):
            records, answers = future.result()
            for index, record in enumerate(records):
                candidates[record["sha256"]] = validated(
                    record, answers.get(index), allow_new_tags=not args.no_new_tags
                )
            write_candidates(output, candidates)
            progress.advance(len(records))
    progress.finish(f"classified {len(candidates)} maps into tag candidates")

    if len(candidates) != len(maps):
        raise SystemExit(f"candidate count {len(candidates)} != map count {len(maps)}")
    if args.apply:
        for record in maps:
            candidate = candidates[record["sha256"]]
            record.update(
                tags=candidate["tags"],
                tag_confidence=candidate["tag_confidence"],
                tag_evidence=candidate["tag_evidence"],
                tag_schema_version=TAG_SCHEMA_VERSION,
            )
        save_catalog(root, catalog)

    tags = Counter(tag for value in candidates.values() for tag in value["tags"])
    confidence = Counter(value["tag_confidence"] for value in candidates.values())
    extensions = Counter(
        tag for value in candidates.values() for tag in value["tags"] if tag not in ALLOWED_TAGS
    )
    emit_report(
        {
            "maps": len(maps),
            "candidates": str(output),
            "applied": args.apply,
            "tag_schema_version": TAG_SCHEMA_VERSION,
            "tag_counts": dict(tags.most_common()),
            "confidence": dict(confidence),
            "taxonomy_extensions": dict(extensions.most_common()),
        }
    )

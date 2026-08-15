"""Create resumable, AI-assisted gameplay / franchise / theme tag candidates."""

from __future__ import annotations

import argparse
import json
import os
import urllib.error
import urllib.request
from collections import Counter
from pathlib import Path

from ..catalog import emit_report, load_catalog, save_catalog, write_atomic
from ..progress import Progress
from ..tagging import ALLOWED_TAGS, TAG_SCHEMA_VERSION, map_text, seed_tags, taxonomy_prompt

DEFAULT_MODEL = "mlx-community/Qwen3-4B-Instruct-2507-4bit"
DEFAULT_ENDPOINT = "http://127.0.0.1:8080/v1/chat/completions"


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/maps.json")
    parser.add_argument("--endpoint", default=os.getenv("WAR3_TAG_LLM_ENDPOINT", DEFAULT_ENDPOINT))
    parser.add_argument("--model", default=os.getenv("WAR3_TAG_LLM_MODEL", DEFAULT_MODEL))
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument("--output", type=Path, help="Defaults to catalog/tag-candidates.jsonl")
    parser.add_argument("--limit", type=int, help="Classify only the first N records (smoke testing)")
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


def request_tags(endpoint: str, model: str, batch: list[dict]) -> list[dict]:
    payload = {
        "model": model,
        "temperature": 0,
        # The reply uses compact batch-local indexes instead of 64-char SHA
        # strings.  This leaves enough room for a whole batch while bounding
        # a malformed answer before it turns into a multi-minute generation.
        "max_tokens": min(2048, 64 + 32 * len(batch)),
        "messages": [
            {"role": "system", "content": taxonomy_prompt()},
            {"role": "user", "content": json.dumps({"maps": batch}, ensure_ascii=False)},
        ],
    }
    request = urllib.request.Request(
        endpoint,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=300) as response:
            reply = json.loads(response.read().decode("utf-8"))
    except urllib.error.URLError as error:
        raise SystemExit(f"tag model unavailable at {endpoint}: {error}") from error
    try:
        text = reply["choices"][0]["message"]["content"].strip()
        value = json.loads(text)
        maps = value["maps"]
    except (IndexError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"tag model returned invalid JSON: {reply!r}") from error
    return maps if isinstance(maps, list) else []


def validated(record: dict, response: dict | None) -> dict:
    seeds, evidence = seed_tags(record)
    allowed = [tag for tag in (response or {}).get("tags", []) if tag in ALLOWED_TAGS]
    tags = list(dict.fromkeys(allowed))
    if not any(tag.startswith("玩法:") for tag in tags):
        tags.extend(tag for tag in seeds if tag.startswith("玩法:"))
    if not any(tag.startswith("玩法:") for tag in tags):
        tags.append("玩法:其他")
    confidence = str((response or {}).get("confidence", "low"))
    if confidence not in {"high", "medium", "low"}:
        confidence = "low"
    if response is not None:
        evidence.append("ai:closed_taxonomy_classification")
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
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    all_maps = catalog["maps"]
    if args.limit is not None and args.limit < 1:
        raise SystemExit("--limit must be positive")
    maps = all_maps[: args.limit]
    if args.apply and len(maps) != len(all_maps):
        raise SystemExit("--apply cannot be used with --limit")
    output = candidate_path(root, args.output)
    candidates = load_completed(output)
    known = {str(record.get("sha256", "")) for record in maps}
    candidates = {digest: value for digest, value in candidates.items() if digest in known}

    pending = [record for record in maps if record["sha256"] not in candidates]
    progress = Progress(len(pending), "maps")
    for start in range(0, len(pending), args.batch_size):
        records = pending[start : start + args.batch_size]
        if args.dry_run:
            answers: dict[str, dict] = {}
        else:
            response = request_tags(
                args.endpoint, args.model, [model_input(record, index) for index, record in enumerate(records)]
            )
            answers = {
                int(item["i"]): item
                for item in response
                if isinstance(item, dict) and isinstance(item.get("i"), int)
            }
        for index, record in enumerate(records):
            candidates[record["sha256"]] = validated(record, answers.get(index))
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
    emit_report(
        {
            "maps": len(maps),
            "candidates": str(output),
            "applied": args.apply,
            "tag_schema_version": TAG_SCHEMA_VERSION,
            "tag_counts": dict(tags.most_common()),
            "confidence": dict(confidence),
        }
    )

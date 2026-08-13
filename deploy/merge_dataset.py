#!/usr/bin/env python3
"""Merge one parsed catalog batch into a content-addressed dataset."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import tempfile
import time
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", type=Path, help="Published dataset root to update")
    parser.add_argument("batch", type=Path, help="Parsed batch root to merge")
    parser.add_argument("--collection", required=True, help="Source collection shown on the website")
    parser.add_argument("--target-collection", help="Fill missing collection on existing target maps")
    parser.add_argument(
        "--replace-failures",
        action="store_true",
        help="Discard target failures after a corrected full re-scan",
    )
    return parser.parse_args()


def load_catalog(root: Path) -> dict:
    path = root / "catalog" / "maps.json"
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or not isinstance(value.get("maps"), list):
        raise SystemExit(f"invalid catalog: {path}")
    return value


def write_atomic(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def object_path(root: Path, item: dict) -> Path:
    relative = Path(str(item.get("dataset_path", "")))
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise SystemExit(f"dataset_path escapes root: {relative}") from error
    return path


def merge_sources(existing: dict, incoming: dict) -> None:
    sources = {str(value) for value in existing.get("source_paths", [])}
    sources.update(str(value) for value in incoming.get("source_paths", []))
    existing["source_paths"] = sorted(sources)
    collections = {str(value) for value in existing.get("collections", []) if value}
    collections.update(str(value) for value in incoming.get("collections", []) if value)
    if existing.get("collection"):
        collections.add(str(existing["collection"]))
    if incoming.get("collection"):
        collections.add(str(incoming["collection"]))
    existing["collections"] = sorted(collections)


REFRESH_FIELDS = (
    "name",
    "author",
    "description",
    "recommended_players",
    "max_players",
    "player_count",
    "category",
    "filename",
    "extension",
    "format",
    "content_type",
    "size",
    "dataset_path",
    "download_url",
    "cover_data",
    "cover_source",
    "cover_status",
    "format_version",
    "tileset",
    "playable_width",
    "playable_height",
    "parse_status",
    "parse_error",
)


def refresh_record(existing: dict, incoming: dict) -> None:
    for field in REFRESH_FIELDS:
        if field in incoming:
            existing[field] = incoming[field]


def main() -> None:
    args = parse_args()
    target = args.target.resolve()
    batch = args.batch.resolve()
    catalog = load_catalog(target)
    batch_catalog = load_catalog(batch)

    existing_by_sha: dict[str, dict] = {}
    for item in catalog["maps"]:
        digest = str(item.get("sha256", ""))
        if args.target_collection and not item.get("collection"):
            item["collection"] = args.target_collection
        item["collections"] = sorted(
            {str(value) for value in item.get("collections", []) if value}
            | ({str(item["collection"])} if item.get("collection") else set())
        )
        existing_by_sha[digest] = item

    added = 0
    duplicates = 0
    refreshed = 0
    for incoming_value in batch_catalog["maps"]:
        incoming = dict(incoming_value)
        incoming["collection"] = args.collection
        incoming["collections"] = [args.collection]
        digest = str(incoming.get("sha256", ""))
        source = object_path(batch, incoming)
        if not source.is_file():
            raise SystemExit(f"missing batch object: {source}")
        if source.stat().st_size != int(incoming.get("size") or -1):
            raise SystemExit(f"batch object size mismatch: {source}")
        if digest in existing_by_sha:
            merge_sources(existing_by_sha[digest], incoming)
            refresh_record(existing_by_sha[digest], incoming)
            duplicates += 1
            refreshed += 1
            continue
        destination = object_path(target, incoming)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists() and destination.stat().st_size != source.stat().st_size:
            raise SystemExit(f"existing object size mismatch: {destination}")
        if not destination.exists():
            shutil.copy2(source, destination)
        existing_by_sha[digest] = incoming
        added += 1

    maps = sorted(existing_by_sha.values(), key=lambda item: str(item.get("sha256", "")))
    campaign_count = sum(item.get("content_type") == "campaign" for item in maps)
    catalog.update(
        schema_version=max(int(catalog.get("schema_version") or 1), 2),
        generated_at_unix=int(time.time()),
        map_count=len(maps),
        playable_map_count=len(maps) - campaign_count,
        campaign_count=campaign_count,
        source_count=sum(len(item.get("source_paths", [])) for item in maps),
        total_bytes=sum(int(item.get("size") or 0) for item in maps),
        maps=maps,
    )
    write_atomic(target / "catalog" / "maps.json", json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    write_atomic(
        target / "catalog" / "maps.jsonl",
        "".join(json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n" for item in maps),
    )

    failures = []
    failure_paths = [batch / "catalog" / "failures.json"]
    if not args.replace_failures:
        failure_paths.insert(0, target / "catalog" / "failures.json")
    for path in failure_paths:
        if path.is_file():
            values = json.loads(path.read_text(encoding="utf-8"))
            if isinstance(values, list):
                failures.extend(values)
    failures = list({json.dumps(value, ensure_ascii=False, sort_keys=True): value for value in failures}.values())
    write_atomic(target / "catalog" / "failures.json", json.dumps(failures, ensure_ascii=False, indent=2) + "\n")
    print(
        f"merged {args.collection}: added={added}, refreshed={refreshed}, "
        f"total={len(maps)}, failures={len(failures)}"
    )


if __name__ == "__main__":
    main()

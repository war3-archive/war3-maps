"""Merge one parsed catalog batch into a content-addressed dataset."""

from __future__ import annotations

import argparse
import json
import shutil
import time
from pathlib import Path

from ..catalog import (
    emit_report,
    failures_path,
    load_catalog,
    object_path,
    save_catalog,
    write_atomic,
)
from ..progress import track

#: Fields a re-parse of the same bytes can improve. Provenance — collection,
#: source paths — is merged separately and never overwritten wholesale.
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


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("target", type=Path, help="Published dataset root to update")
    parser.add_argument("batch", type=Path, help="Parsed batch root to merge")
    parser.add_argument(
        "--collection", required=True, help="Source collection shown on the website"
    )
    parser.add_argument(
        "--target-collection", help="Fill missing collection on existing target maps"
    )
    parser.add_argument(
        "--replace-failures",
        action="store_true",
        help="Discard target failures after a corrected full re-scan",
    )


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


def refresh_record(existing: dict, incoming: dict) -> None:
    for field in REFRESH_FIELDS:
        if field in incoming:
            existing[field] = incoming[field]


def collect_failures(target: Path, batch: Path, replace: bool) -> list:
    paths = [failures_path(batch)]
    if not replace:
        paths.insert(0, failures_path(target))
    failures: list = []
    for path in paths:
        if path.is_file():
            values = json.loads(path.read_text(encoding="utf-8"))
            if isinstance(values, list):
                failures.extend(values)
    return list(
        {
            json.dumps(value, ensure_ascii=False, sort_keys=True): value for value in failures
        }.values()
    )


def run(args: argparse.Namespace) -> None:
    target = args.target.resolve()
    batch = args.batch.resolve()
    catalog = load_catalog(target)
    batch_catalog = load_catalog(batch)

    existing_by_sha: dict[str, dict] = {}
    for item in track(
        catalog["maps"], "target maps", note=f"indexed {len(catalog['maps'])} target maps"
    ):
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
    incoming_maps = batch_catalog["maps"]
    for incoming_value in track(
        incoming_maps, "batch maps", note=f"merged {len(incoming_maps)} batch maps"
    ):
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
    )
    save_catalog(target, catalog, maps)

    failures = collect_failures(target, batch, args.replace_failures)
    write_atomic(failures_path(target), json.dumps(failures, ensure_ascii=False, indent=2) + "\n")
    emit_report(
        {
            "collection": args.collection,
            "added": added,
            "duplicates": duplicates,
            "refreshed": refreshed,
            "total": len(maps),
            "failures": len(failures),
        }
    )

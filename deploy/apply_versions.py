#!/usr/bin/env python3
"""Merge `war3-manager scan-versions` output into an existing catalog.

Rebuilding the catalog from scratch would re-derive collection assignments and
source provenance from the batch layout, which no longer exists, so the w3i
version fields are patched in by SHA-256 instead.
"""

from __future__ import annotations

import argparse
import json
import os
import tempfile
from pathlib import Path

FIELDS = ("format_version", "editor_version", "build_version")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("versions", type=Path, help="JSONL from `war3-manager scan-versions`")
    parser.add_argument(
        "--overwrite-format",
        action="store_true",
        help="Also replace format_version (default: only fill it when absent)",
    )
    return parser.parse_args()


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


def main() -> None:
    args = parse_args()
    root = args.dataset_root.resolve()
    catalog_path = root / "catalog" / "maps.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    maps = catalog.get("maps")
    if not isinstance(maps, list):
        raise SystemExit(f"invalid catalog: {catalog_path}")

    scanned: dict[str, dict] = {}
    with args.versions.open(encoding="utf-8") as stream:
        for line in stream:
            line = line.strip()
            if line:
                record = json.loads(line)
                scanned[str(record.get("sha256", ""))] = record

    updated = 0
    missing = 0
    conflicts = 0
    for item in maps:
        record = scanned.get(str(item.get("sha256", "")))
        if record is None:
            missing += 1
            continue
        before = tuple(item.get(field) for field in FIELDS)
        if record.get("format_version") is not None:
            existing = item.get("format_version")
            if existing is None or args.overwrite_format:
                item["format_version"] = record["format_version"]
            elif existing != record["format_version"]:
                conflicts += 1
        item["editor_version"] = record.get("editor_version")
        item["build_version"] = record.get("build_version")
        if tuple(item.get(field) for field in FIELDS) != before:
            updated += 1

    write_atomic(catalog_path, json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    write_atomic(
        root / "catalog" / "maps.jsonl",
        "".join(json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n" for item in maps),
    )
    with_editor = sum(1 for item in maps if item.get("editor_version") is not None)
    with_build = sum(1 for item in maps if item.get("build_version"))
    print(
        json.dumps(
            {
                "maps": len(maps),
                "updated": updated,
                "not_scanned": missing,
                "format_version_conflicts": conflicts,
                "with_editor_version": with_editor,
                "with_build_version": with_build,
            }
        )
    )


if __name__ == "__main__":
    main()

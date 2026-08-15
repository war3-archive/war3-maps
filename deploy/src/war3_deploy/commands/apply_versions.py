"""Merge `war3-manager scan-versions` output into an existing catalog.

Rebuilding the catalog from scratch would re-derive collection assignments and
source provenance from the batch layout, which no longer exists, so the w3i
version fields are patched in by SHA-256 instead.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from ..catalog import emit_report, load_catalog, load_scan, match_scanned, save_catalog

FIELDS = ("format_version", "editor_version", "build_version")


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("versions", type=Path, help="JSONL from `war3-manager scan-versions`")
    parser.add_argument(
        "--overwrite-format",
        action="store_true",
        help="Also replace format_version (default: only fill it when absent)",
    )


def run(args: argparse.Namespace) -> None:
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    maps = catalog["maps"]
    scanned = load_scan(args.versions)

    updated = 0
    missing = 0
    conflicts = 0
    for item, record in match_scanned(maps, scanned, note=f"applied versions to {len(maps)} maps"):
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

    save_catalog(root, catalog)
    emit_report(
        {
            "maps": len(maps),
            "updated": updated,
            "not_scanned": missing,
            "format_version_conflicts": conflicts,
            "with_editor_version": sum(
                1 for item in maps if item.get("editor_version") is not None
            ),
            "with_build_version": sum(1 for item in maps if item.get("build_version")),
        }
    )

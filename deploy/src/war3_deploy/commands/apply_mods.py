"""Merge `war3-manager scan-mods` output into an existing catalog.

Signatures are updated more often than the dataset is rebuilt, so detection
results are patched in by SHA-256 instead of regenerating the catalog (which
would discard collection assignments and source provenance).

An object missing from the scan keeps whatever it had; an object present in the
scan without a `modification` has its field cleared, so retiring a signature
actually removes the label from the site.
"""

from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path

from ..catalog import emit_report, load_catalog, load_scan, match_scanned, save_catalog


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("mods", type=Path, help="JSONL from `war3-manager scan-mods`")


def run(args: argparse.Namespace) -> None:
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    maps = catalog["maps"]
    scanned = load_scan(args.mods)

    changed = 0
    tools: Counter[str] = Counter()
    for item, record in match_scanned(maps, scanned, note=f"applied mods to {len(maps)} maps"):
        if record is not None:
            before = item.get("modification")
            after = record.get("modification")
            if after is None:
                item.pop("modification", None)
            else:
                item["modification"] = after
            if before != item.get("modification"):
                changed += 1
        modification = item.get("modification")
        if modification:
            tools[f"{modification.get('tool')}:{modification.get('variant') or '?'}"] += 1

    save_catalog(root, catalog)
    emit_report(
        {
            "maps": len(maps),
            "scanned": len(scanned),
            "changed": changed,
            "with_modification": sum(tools.values()),
            "by_variant": dict(tools.most_common()),
        }
    )

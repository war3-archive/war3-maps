#!/usr/bin/env python3
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
import json
import os
import tempfile
from collections import Counter
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("mods", type=Path, help="JSONL from `war3-manager scan-mods`")
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
    with args.mods.open(encoding="utf-8") as stream:
        for line in stream:
            line = line.strip()
            if line:
                record = json.loads(line)
                scanned[str(record.get("sha256", ""))] = record

    changed = 0
    tools = Counter()
    for item in maps:
        record = scanned.get(str(item.get("sha256", "")))
        if record is None:
            continue
        before = item.get("modification")
        after = record.get("modification")
        if after is None:
            item.pop("modification", None)
        else:
            item["modification"] = after
        if before != item.get("modification"):
            changed += 1

    for item in maps:
        modification = item.get("modification")
        if modification:
            tools[f"{modification.get('tool')}:{modification.get('variant') or '?'}"] += 1

    write_atomic(catalog_path, json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    write_atomic(
        root / "catalog" / "maps.jsonl",
        "".join(json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n" for item in maps),
    )
    print(
        json.dumps(
            {
                "maps": len(maps),
                "scanned": len(scanned),
                "changed": changed,
                "with_modification": sum(1 for item in maps if item.get("modification")),
                "by_variant": dict(tools.most_common()),
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()

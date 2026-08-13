#!/usr/bin/env python3
"""Merge `war3-manager rescan` output into an existing catalog.

Parser fixes keep turning previously unreadable maps into readable ones, so the
metadata a record derives from the file itself is refreshed here. Provenance
(collection, source paths, dataset path, download URL) is derived from the batch
layout that no longer exists and is never touched.

Covers recovered by the rescan land as `covers/<xx>/<sha>.png`; run
`deploy/export_covers.py` afterwards to encode them and set `cover_path` /
`cover_url`.
"""

from __future__ import annotations

import argparse
import json
import os
import tempfile
from collections import Counter
from pathlib import Path

# Fields owned by the file itself. Everything else in a record is provenance.
DERIVED = (
    "name",
    "author",
    "description",
    "recommended_players",
    "max_players",
    "player_count",
    "format_version",
    "editor_version",
    "build_version",
    "tileset",
    "playable_width",
    "playable_height",
    "parse_status",
    "parse_error",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("rescan", type=Path, help="JSONL from `war3-manager rescan`")
    parser.add_argument(
        "--keep-names",
        action="store_true",
        help="Do not replace an existing name/author/description (only fill blanks)",
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
    with args.rescan.open(encoding="utf-8") as stream:
        for line in stream:
            line = line.strip()
            if line:
                record = json.loads(line)
                scanned[str(record.get("sha256", ""))] = record

    outcomes = Counter()
    renamed = []
    for item in maps:
        record = scanned.get(str(item.get("sha256", "")))
        if record is None:
            outcomes["not_scanned"] += 1
            continue

        was_broken = item.get("parse_status") != "ok"
        before_name = item.get("name")

        for field in DERIVED:
            if field not in record:
                # parse_error is absent from the JSONL when parsing succeeded.
                if field == "parse_error":
                    item.pop(field, None)
                continue
            if args.keep_names and field in ("name", "author", "description") and item.get(field):
                continue
            item[field] = record[field]

        modification = record.get("modification")
        if modification is None:
            item.pop("modification", None)
        else:
            item["modification"] = modification

        if record.get("cover_status") == "ok":
            item["cover_status"] = "ok"
        if record.get("cover_source"):
            item["cover_source"] = record["cover_source"]

        if was_broken and item.get("parse_status") == "ok":
            outcomes["recovered"] += 1
            if before_name != item.get("name"):
                renamed.append({"sha256": item["sha256"], "from": before_name, "to": item["name"]})
        elif item.get("parse_status") == "ok":
            outcomes["ok"] += 1
        else:
            outcomes["still_broken"] += 1

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
                "outcomes": dict(outcomes),
                "with_modification": sum(1 for item in maps if item.get("modification")),
                "renamed_examples": renamed[:5],
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()

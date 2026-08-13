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
    "name_source",
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


def strip_warcraft_codes(text: str) -> str:
    """Mirror of `catalog.rs::strip_warcraft_codes` for names repaired here."""
    out = []
    index = 0
    while index < len(text):
        char = text[index]
        if char != "|" or index + 1 >= len(text):
            out.append(char)
            index += 1
            continue
        marker = text[index + 1]
        if marker in "cC" and len(text) >= index + 10 and all(
            c in "0123456789abcdefABCDEF" for c in text[index + 2 : index + 10]
        ):
            index += 10
        elif marker in "rR":
            index += 2
        elif marker in "nN":
            out.append(" ")
            index += 2
        elif marker == "|":
            out.append("|")
            index += 2
        else:
            out.append(char)
            index += 1
    return " ".join("".join(out).split())


def name_from_filename(item: dict) -> str | None:
    """The map's own filename, which the catalog preserves as provenance.

    `rescan` walks the content-addressed `objects/` tree, so the filename it
    sees is the sha256 and its filename-tier name is a hash, not a title. The
    original name survives in the record, so repair from there instead.
    """
    filename = item.get("filename")
    if not isinstance(filename, str) or not filename:
        return None
    stem = Path(filename).stem
    cleaned = strip_warcraft_codes(stem)
    return cleaned or None


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
            if field == "name" and record.get("name_source") == "filename":
                # Would write the sha256 over the record's real title.
                repaired = name_from_filename(item)
                if repaired is not None:
                    item["name"] = repaired
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

        # A map that stays unreadable can still gain a real name, because the
        # HM3W tier does not depend on the archive opening. Count those too, or
        # a run that recovers hundreds of titles reports as a no-op.
        if before_name != item.get("name"):
            outcomes["renamed"] += 1
            renamed.append({"sha256": item["sha256"], "from": before_name, "to": item["name"]})

        if was_broken and item.get("parse_status") == "ok":
            outcomes["recovered"] += 1
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

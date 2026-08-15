"""Merge `war3-manager rescan` output into an existing catalog.

Parser fixes keep turning previously unreadable maps into readable ones, so the
metadata a record derives from the file itself is refreshed here. Provenance
(collection, source paths, dataset path, download URL) is derived from the batch
layout that no longer exists and is never touched.

Covers recovered by the rescan land as `covers/<xx>/<sha>.png`; run
`war3-deploy export-covers` afterwards to encode them and set `cover_path` /
`cover_url`.
"""

from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path

from ..catalog import emit_report, load_catalog, load_scan, match_scanned, save_catalog

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


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing catalog/")
    parser.add_argument("rescan", type=Path, help="JSONL from `war3-manager rescan`")
    parser.add_argument(
        "--keep-names",
        action="store_true",
        help="Do not replace an existing name/author/description (only fill blanks)",
    )


def usable_metadata(status: object) -> bool:
    """A record has usable map metadata, even if the archive itself is opaque."""
    return status in {"ok", "carved"}


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
        if (
            marker in "cC"
            and len(text) >= index + 10
            and all(c in "0123456789abcdefABCDEF" for c in text[index + 2 : index + 10])
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


def run(args: argparse.Namespace) -> None:
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    maps = catalog["maps"]
    scanned = load_scan(args.rescan)

    outcomes: Counter[str] = Counter()
    renamed = []
    for item, record in match_scanned(maps, scanned, note=f"applied rescan to {len(maps)} maps"):
        if record is None:
            outcomes["not_scanned"] += 1
            continue

        was_broken = not usable_metadata(item.get("parse_status"))
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

        if was_broken and usable_metadata(item.get("parse_status")):
            outcomes["recovered"] += 1
        elif usable_metadata(item.get("parse_status")):
            outcomes["ok"] += 1
        else:
            outcomes["still_broken"] += 1

    save_catalog(root, catalog)
    emit_report(
        {
            "maps": len(maps),
            "scanned": len(scanned),
            "outcomes": dict(outcomes),
            "with_modification": sum(1 for item in maps if item.get("modification")),
            "renamed_examples": renamed[:5],
        }
    )

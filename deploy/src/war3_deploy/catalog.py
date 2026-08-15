"""Reading and rewriting the dataset catalog.

Every command here works on the same layout — `objects/`, `covers/` and a
`catalog/` holding `maps.json`, `maps.jsonl` and `failures.json` — so the
loading, the atomic rewrite of both catalog encodings and the by-SHA join
against a scanner's JSONL live here instead of in each command.
"""

from __future__ import annotations

import json
import os
import tempfile
from collections.abc import Iterator
from pathlib import Path

from .progress import byte_progress, track

#: Hugging Face's raw-file endpoint, which the catalog's `download_url` and
#: `cover_url` point at.
RESOLVE = "https://huggingface.co/datasets/{repo}/resolve/{revision}/{path}"


def catalog_path(root: Path) -> Path:
    return root / "catalog" / "maps.json"


def jsonl_path(root: Path) -> Path:
    return root / "catalog" / "maps.jsonl"


def failures_path(root: Path) -> Path:
    return root / "catalog" / "failures.json"


def write_atomic(path: Path, text: str) -> None:
    """Replace `path` in one step, so an interrupted run leaves it readable."""
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


def load_catalog(root: Path) -> dict:
    """Load `catalog/maps.json`, insisting on the `{"maps": [...]}` shape."""
    path = catalog_path(root)
    if not path.is_file():
        raise SystemExit(f"missing catalog: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or not isinstance(value.get("maps"), list):
        raise SystemExit(f"invalid catalog: {path}")
    return value


def save_catalog(root: Path, catalog: dict, maps: list[dict] | None = None) -> None:
    """Rewrite both catalog encodings from one in-memory catalog.

    `maps.jsonl` is what the Hub's dataset viewer reads and `maps.json` is what
    the site build reads; letting them drift would show two different archives.
    """
    if maps is not None:
        catalog["maps"] = maps
    records: list[dict] = catalog["maps"]
    lines = [
        json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n"
        for item in track(records, "records", note=f"serialised {len(records)} records")
    ]
    write_atomic(catalog_path(root), json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    write_atomic(jsonl_path(root), "".join(lines))


def load_scan(path: Path) -> dict[str, dict]:
    """Index a scanner's JSONL output by SHA-256.

    A full rescan is hundreds of megabytes, so the pass reports against the
    file size rather than a line count nobody knows before reading it.
    """
    progress = byte_progress(path.stat().st_size)
    records: dict[str, dict] = {}
    with path.open(encoding="utf-8") as stream:
        for line in stream:
            progress.advance(len(line.encode("utf-8")))
            line = line.strip()
            if line:
                record = json.loads(line)
                records[str(record.get("sha256", ""))] = record
    progress.finish(f"read {len(records)} scan records from {path.name}")
    return records


def match_scanned(
    maps: list[dict], scanned: dict[str, dict], note: str | None = None
) -> Iterator[tuple[dict, dict | None]]:
    """Walk the catalog beside the scan, pairing records by SHA-256.

    The record is `None` for a map the scan never saw — every backfill treats
    that as "leave it alone" rather than as a deletion.
    """
    for item in track(maps, "maps", note=note or f"matched {len(maps)} maps"):
        yield item, scanned.get(str(item.get("sha256", "")))


def object_path(root: Path, item: dict) -> Path:
    """Resolve a record's `dataset_path`, refusing anything outside the root."""
    relative = Path(str(item.get("dataset_path", "")))
    path = (root / relative).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise SystemExit(f"dataset_path escapes root: {relative}") from error
    return path


def resolve_url(repo: str, revision: str, path: str) -> str:
    return RESOLVE.format(repo=repo, revision=revision, path=path)


def emit_report(report: dict) -> None:
    """Print a command's summary as JSON on stdout, away from the progress."""
    print(json.dumps(report, ensure_ascii=False, indent=2))

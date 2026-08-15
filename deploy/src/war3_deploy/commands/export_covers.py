"""Export catalog cover thumbnails into a content-addressed covers/ tree.

Covers are stored as WebP next to objects/ in the dataset root and referenced
from the catalog by `cover_path` / `cover_url`, so the catalog itself stays
small enough to fetch from a browser. Inline `cover_data` is dropped.

Sources, in order of preference: an existing covers/<xx>/<sha>.webp (kept),
an existing covers/<xx>/<sha>.png (re-encoded), or inline base64 `cover_data`.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import io
import os
import tempfile
import time
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

from ..catalog import emit_report, load_catalog, resolve_url, save_catalog
from ..progress import map_parallel


@dataclass(frozen=True)
class Options:
    """The encoder settings, separated from argparse so `export_one` is callable."""

    root: Path
    repo_id: str
    revision: str
    quality: int
    max_edge: int
    keep_png: bool


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing objects/ and catalog/")
    parser.add_argument("--repo-id", default="magicwenli/war3-maps", help="owner/dataset-name")
    parser.add_argument("--revision", default="main")
    parser.add_argument("--quality", type=int, default=78)
    parser.add_argument("--max-edge", type=int, default=768, help="Downscale longest edge to this")
    parser.add_argument("--workers", type=int, default=max(4, (os.cpu_count() or 4)))
    parser.add_argument(
        "--keep-png", action="store_true", help="Keep source PNGs after re-encoding"
    )


def encode(source: Image.Image, target: Path, quality: int, max_edge: int) -> None:
    image = source
    if image.mode not in ("RGB", "RGBA"):
        image = image.convert("RGBA" if "A" in image.getbands() else "RGB")
    longest = max(image.size)
    if max_edge and longest > max_edge:
        scale = max_edge / longest
        size = (max(1, round(image.width * scale)), max(1, round(image.height * scale)))
        image = image.resize(size, Image.LANCZOS)
    target.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    os.close(fd)
    try:
        image.save(temporary, format="WEBP", quality=quality, method=4)
        os.replace(temporary, target)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def export_one(item: dict, options: Options) -> tuple[dict, str]:
    digest = str(item.get("sha256", ""))
    shard = digest[:2]
    webp = options.root / "covers" / shard / f"{digest}.webp"
    png = options.root / "covers" / shard / f"{digest}.png"

    outcome = "kept"
    if not webp.is_file():
        if png.is_file():
            with Image.open(png) as image:
                encode(image, webp, options.quality, options.max_edge)
            outcome = "encoded"
        elif item.get("cover_data"):
            try:
                raw = base64.b64decode(str(item["cover_data"]), validate=True)
            except (binascii.Error, ValueError):
                # The Dataset Card intentionally does not expose the bulky
                # inline source field. Invalid data cannot become a WebP, but
                # it must still be removed so every JSONL row conforms to the
                # pinned viewer schema.
                item.pop("cover_data", None)
                item.pop("cover_path", None)
                item.pop("cover_url", None)
                return item, "bad-data"
            with Image.open(io.BytesIO(raw)) as image:
                encode(image, webp, options.quality, options.max_edge)
            outcome = "encoded"
        else:
            item.pop("cover_data", None)
            item.pop("cover_path", None)
            item.pop("cover_url", None)
            return item, "none"

    if png.is_file() and not options.keep_png:
        png.unlink()

    relative = f"covers/{shard}/{digest}.webp"
    item.pop("cover_data", None)
    item["cover_path"] = relative
    item["cover_url"] = resolve_url(options.repo_id, options.revision, relative)
    return item, outcome


def run(args: argparse.Namespace) -> None:
    root = args.dataset_root.resolve()
    catalog = load_catalog(root)
    maps = catalog["maps"]
    options = Options(
        root=root,
        repo_id=args.repo_id,
        revision=args.revision,
        quality=args.quality,
        max_edge=args.max_edge,
        keep_png=args.keep_png,
    )

    results = map_parallel(
        lambda item: export_one(item, options),
        maps,
        "covers",
        args.workers,
        note=f"exported {len(maps)} covers",
    )
    counts: Counter[str] = Counter(outcome for _, outcome in results)

    catalog["generated_at_unix"] = int(time.time())
    save_catalog(root, catalog, [item for item, _ in results])
    emit_report({"covers": dict(counts), "maps": len(catalog["maps"])})

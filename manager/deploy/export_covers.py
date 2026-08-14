#!/usr/bin/env python3
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
import json
import os
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image

RESOLVE = "https://huggingface.co/datasets/{repo}/resolve/{revision}/{path}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("dataset_root", type=Path, help="Root containing objects/ and catalog/")
    parser.add_argument("--repo-id", default="magicwenli/war3-maps", help="owner/dataset-name")
    parser.add_argument("--revision", default="main")
    parser.add_argument("--quality", type=int, default=78)
    parser.add_argument("--max-edge", type=int, default=768, help="Downscale longest edge to this")
    parser.add_argument("--workers", type=int, default=max(4, (os.cpu_count() or 4)))
    parser.add_argument("--keep-png", action="store_true", help="Keep source PNGs after re-encoding")
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


def export_one(item: dict, root: Path, args: argparse.Namespace) -> tuple[dict, str]:
    digest = str(item.get("sha256", ""))
    shard = digest[:2]
    webp = root / "covers" / shard / f"{digest}.webp"
    png = root / "covers" / shard / f"{digest}.png"

    outcome = "kept"
    if not webp.is_file():
        if png.is_file():
            with Image.open(png) as image:
                encode(image, webp, args.quality, args.max_edge)
            outcome = "encoded"
        elif item.get("cover_data"):
            try:
                raw = base64.b64decode(str(item["cover_data"]), validate=True)
            except (binascii.Error, ValueError):
                return item, "bad-data"
            with Image.open(io.BytesIO(raw)) as image:
                encode(image, webp, args.quality, args.max_edge)
            outcome = "encoded"
        else:
            item.pop("cover_data", None)
            item.pop("cover_path", None)
            item.pop("cover_url", None)
            return item, "none"

    if png.is_file() and not args.keep_png:
        png.unlink()

    relative = f"covers/{shard}/{digest}.webp"
    item.pop("cover_data", None)
    item["cover_path"] = relative
    item["cover_url"] = RESOLVE.format(repo=args.repo_id, revision=args.revision, path=relative)
    return item, outcome


def main() -> None:
    args = parse_args()
    root = args.dataset_root.resolve()
    catalog_path = root / "catalog" / "maps.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    maps = catalog.get("maps")
    if not isinstance(maps, list):
        raise SystemExit(f"invalid catalog: {catalog_path}")

    counts: dict[str, int] = {}
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        results = list(pool.map(lambda item: export_one(item, root, args), maps))
    for _, outcome in results:
        counts[outcome] = counts.get(outcome, 0) + 1

    catalog["maps"] = [item for item, _ in results]
    catalog["generated_at_unix"] = int(time.time())
    write_atomic(catalog_path, json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    write_atomic(
        root / "catalog" / "maps.jsonl",
        "".join(
            json.dumps(item, ensure_ascii=False, separators=(",", ":")) + "\n"
            for item in catalog["maps"]
        ),
    )
    print(json.dumps({"covers": counts, "maps": len(catalog["maps"])}, ensure_ascii=False))


if __name__ == "__main__":
    main()

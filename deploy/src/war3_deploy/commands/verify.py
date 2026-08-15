"""Final audit for the published map archive.

Checks the local dataset, the Hugging Face dataset, and the GitHub Pages site
against the expected 17 non-paywall collections, and optionally verifies every
single-file download URL.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from urllib.request import Request, urlopen

from ..catalog import emit_report, failures_path, load_catalog
from ..progress import map_parallel, track

EXPECTED_COLLECTIONS = {
    "战役包",
    "休闲+小游戏",
    "未分类地图",
    "生存图",
    "生存对抗类",
    "恐怖解密图",
    "角色扮演",
    "火影系列地图",
    "火龙地图",
    "肥羊地图合集",
    "防守图",
    "对抗地图",
    "单人地图",
    "TD塔防",
    "ORPG",
    "AGC东方幻想系列",
    "颜色图",
}

USER_AGENT = "war3parser-verify/1"


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--dataset", type=Path, default=Path("/Volumes/APFS/war3-maps-dataset"))
    parser.add_argument("--hf-repo", default="magicwenli/war3-maps")
    parser.add_argument("--site", default="https://war3-archive.github.io/war3-maps/")
    parser.add_argument("--check-urls", action="store_true", help="HEAD-check every download URL")
    parser.add_argument("--check-covers", action="store_true", help="HEAD-check every cover URL")
    parser.add_argument("--url-workers", type=int, default=32)


def fetch_json(url: str, timeout: int = 60) -> dict:
    request = Request(url, headers={"User-Agent": USER_AGENT})
    with urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def head_ok(url: str) -> tuple[str, bool]:
    request = Request(url, method="HEAD", headers={"User-Agent": USER_AGENT})
    try:
        with urlopen(request, timeout=30) as response:
            return url, response.status == 200
    except Exception:
        return url, False


class LocalAudit:
    """Everything one pass over the local catalog can answer.

    Twenty thousand records each cost a `stat` for their cover, so the checks
    share a single walk instead of a comprehension apiece.
    """

    def __init__(self, root: Path, maps: list[dict]) -> None:
        self.collections: Counter[str] = Counter()
        self.inline = 0
        self.covers = 0
        self.bad_cover_source = 0
        self.missing_covers: list[str] = []
        self.missing_urls = 0
        self.download_urls: list[str] = []
        self.cover_urls: list[str] = []
        for item in track(maps, "maps", note=f"audited {len(maps)} local records"):
            self.collections[item.get("collection") or item.get("category") or "未分类"] += 1
            if item.get("cover_data"):
                self.inline += 1
            cover_path = item.get("cover_path")
            if cover_path:
                self.covers += 1
                if item.get("cover_source") not in ("preview", "map"):
                    self.bad_cover_source += 1
                if not (root / cover_path).is_file():
                    self.missing_covers.append(cover_path)
            if item.get("download_url"):
                self.download_urls.append(item["download_url"])
            else:
                self.missing_urls += 1
            if item.get("cover_url"):
                self.cover_urls.append(item["cover_url"])


def run(args: argparse.Namespace) -> None:
    root = args.dataset
    local = load_catalog(root)
    failures = (
        json.loads(failures_path(root).read_text(encoding="utf-8"))
        if failures_path(root).is_file()
        else []
    )

    audit = LocalAudit(root, local["maps"])
    problems: list[str] = []
    missing = EXPECTED_COLLECTIONS - set(audit.collections)
    if missing:
        problems.append(f"missing collections: {sorted(missing)}")
    extra = set(audit.collections) - EXPECTED_COLLECTIONS
    if extra:
        problems.append(f"unexpected collections: {sorted(extra)}")
    paywall = {name for name in audit.collections if "氪金" in name or "学习版" in name}
    if paywall:
        problems.append(f"paywall collections present: {sorted(paywall)}")

    # Covers are files under covers/, referenced by cover_path; inline cover_data
    # was dropped in catalog v2 so the catalog stays browser-fetchable.
    if audit.inline:
        problems.append(f"{audit.inline} records still carry inline cover_data")
    if audit.bad_cover_source:
        problems.append(f"{audit.bad_cover_source} covers have invalid cover_source")
    if audit.missing_covers:
        problems.append(
            f"{len(audit.missing_covers)} cover files missing locally ({audit.missing_covers[:3]})"
        )

    try:
        hf = fetch_json(
            f"https://huggingface.co/datasets/{args.hf_repo}/resolve/main/catalog/maps.json?verify=1"
        )
        hf_count = hf.get("map_count")
    except Exception as error:
        hf_count = None
        problems.append(f"HF catalog fetch failed: {error}")
    if hf_count != local.get("map_count"):
        problems.append(f"HF map_count={hf_count} != local {local.get('map_count')}")

    # The site publishes overview.json (counts + categories) plus paged category
    # shards; there is no full maps.json on the site any more.
    try:
        site = fetch_json(args.site.rstrip("/") + "/data/overview.json?verify=1")
        site_count = site.get("map_count")
        site_covers = site.get("cover_count")
    except Exception as error:
        site_count = None
        site_covers = None
        problems.append(f"site overview fetch failed: {error}")
    if site_count != local.get("map_count"):
        problems.append(f"site map_count={site_count} != local {local.get('map_count')}")
    if site_covers is not None and site_covers != audit.covers:
        problems.append(f"site cover_count={site_covers} != local {audit.covers}")

    def head_all(urls: list[str], label: str) -> None:
        bad = [
            url
            for url, ok in map_parallel(
                head_ok,
                urls,
                f"{label} URLs",
                args.url_workers,
                note=f"checked {len(urls)} {label} URLs",
            )
            if not ok
        ]
        if bad:
            problems.append(f"{len(bad)} {label} URLs failed ({bad[:5]})")

    if args.check_urls:
        if audit.missing_urls:
            problems.append(f"{audit.missing_urls} records lack download_url")
        head_all(audit.download_urls, "download")

    if args.check_covers:
        head_all(audit.cover_urls, "cover")

    emit_report(
        {
            "schema_version": local.get("schema_version"),
            "map_count": local.get("map_count"),
            "playable_map_count": local.get("playable_map_count"),
            "campaign_count": local.get("campaign_count"),
            "source_count": local.get("source_count"),
            "total_bytes": local.get("total_bytes"),
            "collections": dict(audit.collections.most_common()),
            "failures": len(failures),
            "cover_records": audit.covers,
            "hf_map_count": hf_count,
            "site_map_count": site_count,
            "problems": problems,
        }
    )
    if problems:
        raise SystemExit(1)

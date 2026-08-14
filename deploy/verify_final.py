#!/usr/bin/env python3
"""Final audit for the published map archive.

Checks the local dataset, the Hugging Face dataset, and the GitHub Pages site
against the expected 17 non-paywall collections, and optionally verifies every
single-file download URL.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.request import Request, urlopen


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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dataset", type=Path, default=Path("/Volumes/APFS/war3-maps-dataset"))
    parser.add_argument("--hf-repo", default="magicwenli/war3-maps")
    parser.add_argument("--site", default="https://war3-archive.github.io/war3-maps/")
    parser.add_argument("--check-urls", action="store_true", help="HEAD-check every download URL")
    parser.add_argument("--check-covers", action="store_true", help="HEAD-check every cover URL")
    parser.add_argument("--url-workers", type=int, default=32)
    return parser.parse_args()


def fetch_json(url: str, timeout: int = 60) -> dict:
    request = Request(url, headers={"User-Agent": "war3parser-verify/1"})
    with urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def head_ok(url: str) -> tuple[str, bool]:
    request = Request(url, method="HEAD", headers={"User-Agent": "war3parser-verify/1"})
    try:
        with urlopen(request, timeout=30) as response:
            return url, response.status == 200
    except Exception:
        return url, False


def main() -> None:
    args = parse_args()
    local_path = args.dataset / "catalog" / "maps.json"
    local = json.loads(local_path.read_text(encoding="utf-8"))
    failures_path = args.dataset / "catalog" / "failures.json"
    failures = json.loads(failures_path.read_text(encoding="utf-8")) if failures_path.is_file() else []

    collections: dict[str, int] = {}
    for item in local["maps"]:
        collection = item.get("collection") or item.get("category") or "未分类"
        collections[collection] = collections.get(collection, 0) + 1

    problems: list[str] = []
    missing = EXPECTED_COLLECTIONS - set(collections)
    if missing:
        problems.append(f"missing collections: {sorted(missing)}")
    extra = set(collections) - EXPECTED_COLLECTIONS
    if extra:
        problems.append(f"unexpected collections: {sorted(extra)}")
    paywall = {name for name in collections if "氪金" in name or "学习版" in name}
    if paywall:
        problems.append(f"paywall collections present: {sorted(paywall)}")

    # Covers are files under covers/, referenced by cover_path; inline cover_data
    # was dropped in catalog v2 so the catalog stays browser-fetchable.
    inline = sum(1 for item in local["maps"] if item.get("cover_data"))
    if inline:
        problems.append(f"{inline} records still carry inline cover_data")
    cover_ok = sum(1 for item in local["maps"] if item.get("cover_path"))
    cover_bad_source = sum(
        1
        for item in local["maps"]
        if item.get("cover_path") and item.get("cover_source") not in ("preview", "map")
    )
    if cover_bad_source:
        problems.append(f"{cover_bad_source} covers have invalid cover_source")
    cover_missing = [
        item["cover_path"]
        for item in local["maps"]
        if item.get("cover_path") and not (args.dataset / item["cover_path"]).is_file()
    ]
    if cover_missing:
        problems.append(f"{len(cover_missing)} cover files missing locally ({cover_missing[:3]})")

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
    if site_covers is not None and site_covers != cover_ok:
        problems.append(f"site cover_count={site_covers} != local {cover_ok}")

    def head_all(urls: list[str], label: str) -> None:
        bad: list[str] = []
        with ThreadPoolExecutor(max_workers=args.url_workers) as pool:
            for url, ok in pool.map(head_ok, urls):
                if not ok:
                    bad.append(url)
        if bad:
            problems.append(f"{len(bad)} {label} URLs failed ({bad[:5]})")

    if args.check_urls:
        missing_url = sum(1 for item in local["maps"] if not item.get("download_url"))
        if missing_url:
            problems.append(f"{missing_url} records lack download_url")
        head_all([item["download_url"] for item in local["maps"] if item.get("download_url")], "download")

    if args.check_covers:
        head_all([item["cover_url"] for item in local["maps"] if item.get("cover_url")], "cover")

    report = {
        "schema_version": local.get("schema_version"),
        "map_count": local.get("map_count"),
        "playable_map_count": local.get("playable_map_count"),
        "campaign_count": local.get("campaign_count"),
        "source_count": local.get("source_count"),
        "total_bytes": local.get("total_bytes"),
        "collections": dict(sorted(collections.items(), key=lambda pair: -pair[1])),
        "failures": len(failures),
        "cover_records": cover_ok,
        "hf_map_count": hf_count,
        "site_map_count": site_count,
        "problems": problems,
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if problems:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

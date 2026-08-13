#!/usr/bin/env python3
"""Assemble the catalog site into one GitHub Pages artifact."""

from __future__ import annotations

import argparse
import json
import shutil
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import quote


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--catalog", type=Path, help="Local maps.json or maps.jsonl")
    parser.add_argument("--catalog-url", help="Fallback URL for maps.json or maps.jsonl")
    parser.add_argument("--site", type=Path, default=Path("deploy/site"))
    parser.add_argument("--out", type=Path, default=Path("site-dist"))
    parser.add_argument("--hf-repo", help="Hugging Face dataset id, e.g. owner/war3maps")
    parser.add_argument("--revision", default="main")
    return parser.parse_args()


def read_source(path: Path | None, url: str | None) -> str:
    if path and path.is_file():
        return path.read_text(encoding="utf-8")
    if url:
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return response.read().decode("utf-8")
        except (urllib.error.URLError, TimeoutError) as error:
            print(f"warning: catalog URL unavailable ({error}); publishing an empty catalog")
    return '{"schema_version":1,"generated_at":null,"maps":[]}'


def parse_catalog(raw: str) -> dict:
    stripped = raw.lstrip()
    if not stripped:
        return {"schema_version": 1, "generated_at": None, "maps": []}
    if stripped.startswith("[") or stripped.startswith("{"):
        value = json.loads(raw)
        return value if isinstance(value, dict) else {"schema_version": 1, "maps": value}
    return {"schema_version": 1, "maps": [json.loads(line) for line in raw.splitlines() if line.strip()]}


def hf_download_url(repo_id: str, revision: str, dataset_path: str) -> str:
    encoded_path = "/".join(quote(part, safe="") for part in dataset_path.split("/"))
    return f"https://huggingface.co/datasets/{repo_id}/resolve/{quote(revision, safe='')}/{encoded_path}?download=true"


def main() -> None:
    args = parse_args()
    payload = parse_catalog(read_source(args.catalog, args.catalog_url))
    maps = payload.get("maps") or []
    if not isinstance(maps, list):
        raise SystemExit("catalog `maps` must be an array")
    for item in maps:
        if not isinstance(item, dict):
            raise SystemExit("every catalog map must be an object")
        path = item.get("dataset_path")
        if args.hf_repo and path:
            item["download_url"] = hf_download_url(args.hf_repo, args.revision, str(path))

    if args.out.exists():
        shutil.rmtree(args.out)
    shutil.copytree(args.site, args.out)
    data_dir = args.out / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    (data_dir / "maps.json").write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8"
    )
    config = {
        "dataset_repo": args.hf_repo,
        "dataset_url": f"https://huggingface.co/datasets/{args.hf_repo}" if args.hf_repo else None,
        "revision": args.revision,
    }
    (data_dir / "site-config.json").write_text(
        json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (args.out / ".nojekyll").touch()
    print(f"assembled {len(maps)} maps in {args.out}")


if __name__ == "__main__":
    main()

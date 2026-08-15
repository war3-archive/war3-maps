"""Validate and resumably upload a generated map dataset to Hugging Face."""

from __future__ import annotations

import argparse
import hashlib
import os
import time
from pathlib import Path

from ..catalog import load_catalog
from ..progress import Progress, byte_progress


def configure(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("dataset_root", type=Path, help="Root containing objects/ and catalog/")
    parser.add_argument("--repo-id", required=True, help="owner/dataset-name")
    parser.add_argument("--revision", default="main")
    parser.add_argument("--token", default=os.getenv("HF_TOKEN"))
    parser.add_argument("--private", action="store_true", help="Default is a public dataset")
    parser.add_argument(
        "--no-create", action="store_true", help="Require the dataset repo to exist"
    )
    parser.add_argument(
        "--skip-hash-check", action="store_true", help="Skip expensive content re-hashing"
    )
    parser.add_argument("--dry-run", action="store_true")


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate(root: Path, maps: list[dict], check_hash: bool) -> None:
    """Refuse to publish a catalog that does not describe what is on disk.

    Re-hashing reads the whole archive, so that pass reports against total
    bytes: a count of objects moves in lurches when one map is 4 KB and the
    next is 400 MB.
    """
    total_bytes = sum(int(item.get("size") or 0) for item in maps)
    progress = byte_progress(total_bytes) if check_hash else Progress(len(maps), "objects")
    seen: set[str] = set()
    missing: list[str] = []
    for item in maps:
        progress.advance(int(item.get("size") or 0) if check_hash else 1)
        digest = str(item.get("sha256", ""))
        dataset_path = str(item.get("dataset_path", ""))
        if len(digest) != 64 or any(char not in "0123456789abcdef" for char in digest.lower()):
            raise SystemExit(f"invalid sha256 in catalog: {digest!r}")
        if digest in seen:
            raise SystemExit(f"duplicate sha256 in catalog: {digest}")
        seen.add(digest)
        path = (root / dataset_path).resolve()
        try:
            path.relative_to(root)
        except ValueError:
            raise SystemExit(f"dataset_path escapes dataset root: {dataset_path!r}")
        if not dataset_path or not path.is_file():
            missing.append(dataset_path or f"<path missing for {digest}>")
            continue
        cover_path = str(item.get("cover_path", ""))
        if cover_path and not (root / cover_path).is_file():
            missing.append(cover_path)
        expected_size = int(item.get("size") or 0)
        if expected_size and path.stat().st_size != expected_size:
            raise SystemExit(
                f"size mismatch for {dataset_path}: catalog={expected_size}, file={path.stat().st_size}"
            )
        if check_hash and file_sha256(path) != digest.lower():
            raise SystemExit(f"sha256 mismatch for {dataset_path}")
    if missing:
        sample = "\n".join(f"  - {path}" for path in missing[:20])
        raise SystemExit(f"{len(missing)} catalog objects are missing:\n{sample}")
    progress.finish(
        f"validated {len(maps)} objects"
        + (" with content hashes" if check_hash else " (hash check skipped)")
    )


def run(args: argparse.Namespace) -> None:
    root = args.dataset_root.resolve()
    maps = load_catalog(root)["maps"]
    validate(root, maps, check_hash=not args.skip_hash_check)
    total = sum(int(item.get("size") or 0) for item in maps)
    print(f"validated {len(maps)} unique maps ({total} bytes) in {root}")
    if args.dry_run:
        return
    # huggingface_hub reads this setting while importing its tqdm helpers; it
    # therefore has to be cleared before importing the client, not merely
    # before calling upload_folder.
    os.environ.pop("HF_HUB_DISABLE_PROGRESS_BARS", None)
    from huggingface_hub import HfApi, get_token
    from huggingface_hub.utils import enable_progress_bars

    # `hf auth login` stores a token of its own, so only insist on one when the
    # library cannot find any: requiring HF_TOKEN would reject a machine that is
    # already logged in.
    if not args.token and not get_token():
        raise SystemExit("not authenticated: run `hf auth login`, set HF_TOKEN, or pass --token")

    api = HfApi(token=args.token)
    if not args.no_create:
        api.create_repo(args.repo_id, repo_type="dataset", private=args.private, exist_ok=True)
    # `upload_folder` reports through huggingface_hub's own bars.  Explicitly
    # enable them after clearing the environment-level override above.
    enable_progress_bars()
    print(
        f"uploading to {args.repo_id}: hashing against the Hub, then sending what differs",
        flush=True,
    )
    started = time.monotonic()
    # Current huggingface_hub upload_folder uses Xet, adaptive commits and resumes
    # interrupted large-folder uploads when the same command is rerun.
    api.upload_folder(
        repo_id=args.repo_id,
        repo_type="dataset",
        folder_path=str(root),
        revision=args.revision,
        # Covers are published as WebP by export-covers; the PNG masters stay
        # local, and .DS_Store has no business in a public dataset. The patterns
        # are matched with fnmatch against the repo-relative path, so `**/` only
        # matches nested files — the dataset root needs its own bare pattern.
        ignore_patterns=[
            "covers/**/*.png",
            ".DS_Store",
            "**/.DS_Store",
            ".claude",
            ".claude/**",
            "**/.claude",
            "**/.claude/**",
        ],
        commit_message="Upload cleaned Warcraft III map archive",
    )
    elapsed = time.monotonic() - started
    print(
        f"uploaded in {elapsed:.1f}s to https://huggingface.co/datasets/{args.repo_id}",
        flush=True,
    )

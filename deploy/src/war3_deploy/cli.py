"""The `war3-deploy` entry point.

One executable with a subcommand per pass, in the order the release runs them:
merge a batch, back-fill what a rescan learned, export covers, upload, verify.
"""

from __future__ import annotations

import argparse
from collections.abc import Sequence

from . import __version__
from .commands import (
    apply_mods,
    apply_rescan,
    apply_versions,
    export_covers,
    merge,
    upload,
    verify,
)

COMMANDS = (
    ("merge", merge, "Merge a parsed batch into the published dataset"),
    ("apply-rescan", apply_rescan, "Back-fill `war3-manager rescan` output"),
    ("apply-mods", apply_mods, "Back-fill `war3-manager scan-mods` output"),
    ("apply-versions", apply_versions, "Back-fill `war3-manager scan-versions` output"),
    ("export-covers", export_covers, "Encode covers to WebP and link them from the catalog"),
    ("upload", upload, "Validate the dataset and upload it to Hugging Face"),
    ("verify", verify, "Audit the local dataset against what is published"),
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="war3-deploy",
        description="Dataset maintenance and publishing for the Warcraft III map archive.",
    )
    parser.add_argument("--version", action="version", version=f"war3-deploy {__version__}")
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name, module, help_text in COMMANDS:
        subparser = subparsers.add_parser(
            name,
            help=help_text,
            description=module.__doc__,
            formatter_class=argparse.RawDescriptionHelpFormatter,
        )
        module.configure(subparser)
        subparser.set_defaults(run=module.run)
    return parser


def main(argv: Sequence[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    args.run(args)

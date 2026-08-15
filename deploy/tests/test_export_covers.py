from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from war3_deploy.commands.export_covers import Options, export_one


class ExportCoverTests(unittest.TestCase):
    def test_bad_inline_cover_is_not_left_in_dataset_schema(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            options = Options(
                root=Path(temporary),
                repo_id="owner/dataset",
                revision="main",
                quality=78,
                max_edge=768,
                keep_png=False,
            )
            record = {
                "sha256": "a" * 64,
                "cover_data": "not-valid-base64!",
                "cover_path": "covers/stale.webp",
                "cover_url": "https://example.invalid/stale.webp",
            }

            result, outcome = export_one(record, options)

            self.assertEqual(outcome, "bad-data")
            self.assertNotIn("cover_data", result)
            self.assertNotIn("cover_path", result)
            self.assertNotIn("cover_url", result)

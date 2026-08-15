from __future__ import annotations

import unittest

from war3_deploy.commands.merge import refresh_record


class MergeTests(unittest.TestCase):
    def test_refresh_keeps_the_existing_curated_category(self) -> None:
        existing = {"category": "守卫剑阁", "name": "old"}
        incoming = {"category": "未分类", "name": "new"}

        refresh_record(existing, incoming)

        self.assertEqual(existing["category"], "守卫剑阁")
        self.assertEqual(existing["name"], "new")


if __name__ == "__main__":
    unittest.main()

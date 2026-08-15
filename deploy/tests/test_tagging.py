from __future__ import annotations

import unittest

from war3_deploy.commands.classify_tags import validated
from war3_deploy.tagging import map_text, seed_tags, strip_warcraft_codes


class TaggingTests(unittest.TestCase):
    def test_strips_warcraft_codes_before_classifying(self) -> None:
        record = {
            "name": "|cffff0000火影|r|n忍者村大战TD",
            "description": "|cff00ff00僵尸生存|r",
            "collection": "未分类地图",
        }
        self.assertEqual(strip_warcraft_codes(record["name"]), "火影 忍者村大战TD")
        self.assertEqual(map_text(record), ("火影 忍者村大战TD", "僵尸生存"))
        self.assertTrue(
            {"玩法:塔防", "系列:火影忍者", "题材:动漫", "题材:僵尸"}
            <= set(seed_tags(record)[0])
        )

    def test_model_cannot_invent_a_tag_or_remove_gameplay(self) -> None:
        record = {"sha256": "a" * 64, "name": "绿色循环圈TD", "collection": "TD塔防"}
        result = validated(
            record,
            {"tags": ["玩法:塔防", "系列:模型杜撰"], "confidence": "certain"},
        )
        self.assertEqual(result["tags"], ["玩法:塔防"])
        self.assertEqual(result["tag_confidence"], "low")
        self.assertIn("ai:closed_taxonomy_classification", result["tag_evidence"])


if __name__ == "__main__":
    unittest.main()

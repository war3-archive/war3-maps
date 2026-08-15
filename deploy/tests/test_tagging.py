from __future__ import annotations

import json
import unittest
from unittest import mock

from war3_deploy.commands.classify_tags import parse_json_reply, request_tags, validated
from war3_deploy.tagging import (
    canonicalize_tag,
    map_text,
    normalize_extension_tag,
    seed_tags,
    strip_warcraft_codes,
)


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

    def test_model_keeps_gameplay_and_drops_invalid_tag_namespaces(self) -> None:
        record = {"sha256": "a" * 64, "name": "绿色循环圈TD", "collection": "TD塔防"}
        result = validated(
            record,
            {"tags": ["玩法:塔防", "unsafe:模型杜撰"], "confidence": "certain"},
        )
        self.assertEqual(result["tags"], ["玩法:塔防"])
        self.assertEqual(result["tag_confidence"], "low")
        self.assertIn("ai:taxonomy_classification", result["tag_evidence"])

    def test_validated_accepts_short_controlled_taxonomy_extension(self) -> None:
        record = {"sha256": "a" * 64, "name": "忍者动作图", "description": "", "collection": ""}
        result = validated(
            record,
            {"tags": ["玩法:动作", "系列:忍者神龟", "unsafe:ignore", "题材:其他"], "confidence": "high"},
        )
        self.assertEqual(result["tags"], ["玩法:动作", "系列:忍者神龟"])
        self.assertIn("ai:taxonomy_extension:系列:忍者神龟", result["tag_evidence"])

    def test_extension_tag_requires_stable_namespace_and_safe_name(self) -> None:
        self.assertEqual(normalize_extension_tag("题材:克苏鲁"), "题材:克苏鲁")
        self.assertIsNone(normalize_extension_tag("标签:克苏鲁"))
        self.assertIsNone(normalize_extension_tag("题材:其他"))
        self.assertIsNone(normalize_extension_tag("题材:too/much"))

    def test_known_tag_name_is_put_back_in_its_canonical_namespace(self) -> None:
        self.assertEqual(canonicalize_tag("玩法:海战"), "题材:海战")
        self.assertEqual(canonicalize_tag("题材:生存"), "玩法:生存")

    def test_parse_json_reply_accepts_a_markdown_fence(self) -> None:
        self.assertEqual(parse_json_reply("```json\n{\"maps\": []}\n```"), {"maps": []})

    def test_request_tags_sends_bearer_when_api_key_given(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: int = 0):
            captured["headers"] = dict(request.headers)
            captured["payload"] = json.loads(request.data.decode())
            content = json.dumps({"choices": [{"message": {"content": '{"maps":[]}'}}]})

            class FakeResponse:
                def read(self) -> bytes:
                    return content.encode()

                def __enter__(self):
                    return self

                def __exit__(self, *exc) -> None:
                    return None

            return FakeResponse()

        with mock.patch("urllib.request.urlopen", side_effect=fake_urlopen):
            request_tags(
                "https://openrouter.ai/api/v1/chat/completions",
                "deepseek/deepseek-v4-flash-0731",
                [],
                api_key="sk-or-test",
            )
        self.assertEqual(captured["headers"].get("Authorization"), "Bearer sk-or-test")
        self.assertEqual(captured["payload"]["reasoning"], {"enabled": False})

    def test_request_tags_omits_authorization_without_key(self) -> None:
        captured: dict[str, object] = {}

        def fake_urlopen(request, timeout: int = 0):
            captured["headers"] = dict(request.headers)
            captured["payload"] = json.loads(request.data.decode())
            content = json.dumps({"choices": [{"message": {"content": '{"maps":[]}'}}]})

            class FakeResponse:
                def read(self) -> bytes:
                    return content.encode()

                def __enter__(self):
                    return self

                def __exit__(self, *exc) -> None:
                    return None

            return FakeResponse()

        with mock.patch("urllib.request.urlopen", side_effect=fake_urlopen):
            request_tags("http://127.0.0.1:8080/v1/chat/completions", "local-model", [])
        self.assertNotIn("Authorization", captured["headers"])
        self.assertNotIn("reasoning", captured["payload"])


if __name__ == "__main__":
    unittest.main()

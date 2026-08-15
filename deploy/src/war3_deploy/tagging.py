"""Stable taxonomy and input normalisation for AI-assisted map tagging.

The model is never allowed to invent a tag.  It may select only from this
small, versioned vocabulary, which keeps filtering and future model runs
compatible.  Existing ``collection`` / ``category`` values are provenance and
weak hints; title and description are the actual classification input.
"""

from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass

TAG_SCHEMA_VERSION = 1

GAMEPLAY_TAGS = (
    "玩法:塔防",
    "玩法:防守",
    "玩法:生存",
    "玩法:对抗",
    "玩法:RPG",
    "玩法:ORPG",
    "玩法:战役",
    "玩法:解谜",
    "玩法:休闲",
    "玩法:竞速",
    "玩法:微操",
    "玩法:其他",
)

SERIES_TAGS = (
    "系列:DotA",
    "系列:火影忍者",
    "系列:三国",
    "系列:魔兽世界",
    "系列:英雄联盟",
    "系列:仙剑奇侠传",
    "系列:金庸",
    "系列:西游记",
    "系列:封神",
    "系列:东方Project",
    "系列:死神",
    "系列:海贼王",
    "系列:宠物小精灵",
    "系列:星河战队",
)

THEME_TAGS = (
    "题材:仙侠",
    "题材:武侠",
    "题材:神话",
    "题材:魔幻",
    "题材:动漫",
    "题材:科幻",
    "题材:军事",
    "题材:历史",
    "题材:恐怖",
    "题材:僵尸",
    "题材:末日",
)

ALLOWED_TAGS = frozenset((*GAMEPLAY_TAGS, *SERIES_TAGS, *THEME_TAGS))


@dataclass(frozen=True)
class Rule:
    tag: str
    pattern: re.Pattern[str]


def _rule(tag: str, pattern: str) -> Rule:
    return Rule(tag, re.compile(pattern, re.IGNORECASE))


# These are deliberately high-precision seeds, not a replacement for the
# model.  They give the model useful candidates and provide a deterministic
# fallback if an individual request has to be retried.
TITLE_RULES = (
    _rule("玩法:塔防", r"(?:(?<![a-z])td(?![a-z])|塔防|守塔|防御塔|炮塔|循环圈|小偷TD)"),
    _rule("玩法:防守", r"(?:防守|守城|守卫|守护|保卫|守家|守关|守卫剑阁)"),
    _rule("玩法:生存", r"(?:生存|求生|幸存|逃生|survival)"),
    _rule("玩法:对抗", r"(?:(?<![a-z])dota(?![a-z])|(?<![a-z])3c(?![a-z])|对抗|竞技|争霸|对战|(?<![a-z])pvp(?![a-z]))"),
    _rule("玩法:ORPG", r"\borpg\b"),
    _rule("玩法:RPG", r"(?:\brpg\b|角色扮演|修仙|修真|仙侠|武侠|江湖|传奇)"),
    _rule("玩法:战役", r"(?:战役|剧情|序章|终章|章节)"),
    _rule("玩法:解谜", r"(?:解谜|解密|密室|逃脱)"),
    _rule("玩法:竞速", r"(?:竞速|跑酷|快跑|赛跑)"),
    _rule("玩法:微操", r"(?:微操|\bmicro\b)"),
    _rule("玩法:休闲", r"(?:小游戏|躲猫猫|钓鱼|农场)"),
    _rule("系列:DotA", r"(?<![a-z])dota(?![a-z])"),
    _rule("系列:火影忍者", r"(?:火影|naruto|忍者村大战)"),
    _rule("系列:三国", r"(?:三国|真三|三国无双)"),
    _rule("系列:魔兽世界", r"(?:魔兽世界|\bwow\b)"),
    _rule("系列:英雄联盟", r"(?:英雄联盟|\blol\b)"),
    _rule("系列:仙剑奇侠传", r"仙剑"),
    _rule("系列:金庸", r"金庸"),
    _rule("系列:西游记", r"(?:西游|悟空)"),
    _rule("系列:封神", r"封神"),
    _rule("系列:东方Project", r"(?:东方|touhou)"),
    _rule("系列:死神", r"(?:死神|bleach)"),
    _rule("系列:海贼王", r"(?:海贼王|one\s*piece)"),
    _rule("系列:宠物小精灵", r"(?:宠物小精灵|口袋妖怪|pokemon)"),
    _rule("系列:星河战队", r"(?:星河战队|starship\s*troopers)"),
    _rule("题材:仙侠", r"(?:仙侠|修仙|修真|仙界)"),
    _rule("题材:武侠", r"(?:武侠|江湖|侠客|门派)"),
    _rule("题材:神话", r"(?:神话|封神|洪荒|女娲)"),
    _rule("题材:魔幻", r"(?:魔幻|奇幻|魔法|龙族)"),
    _rule("题材:动漫", r"(?:动漫|二次元|火影|死神|海贼|东方)"),
    _rule("题材:科幻", r"(?:科幻|星际|机甲|未来|宇宙)"),
    _rule("题材:军事", r"(?:二战|战争|军团|军港|军队|抗战)"),
    _rule("题材:历史", r"(?:三国|战国|历史|隋唐)"),
    _rule("题材:恐怖", r"(?:恐怖|惊魂|鬼|诅咒|噩梦)"),
    _rule("题材:僵尸", r"(?:僵尸|丧尸|生化)"),
    _rule("题材:末日", r"(?:末日|浩劫|毁灭|灾变)"),
)

LEGACY_GAMEPLAY = {
    "TD塔防": "玩法:塔防",
    "防守图": "玩法:防守",
    "生存图": "玩法:生存",
    "对抗地图": "玩法:对抗",
    "角色扮演": "玩法:RPG",
    "ORPG": "玩法:ORPG",
    "战役包": "玩法:战役",
    "休闲+小游戏": "玩法:休闲",
    "恐怖解密图": "玩法:解谜",
}


def strip_warcraft_codes(value: object) -> str:
    """Remove Warcraft III colour, reset, newline and escaped-pipe controls."""
    text = unicodedata.normalize("NFKC", str(value or ""))
    out: list[str] = []
    index = 0
    while index < len(text):
        if text[index] != "|" or index + 1 >= len(text):
            out.append(text[index])
            index += 1
            continue
        marker = text[index + 1]
        colour = text[index + 2 : index + 10]
        if marker in "cC" and len(colour) == 8 and all(char in "0123456789abcdefABCDEF" for char in colour):
            index += 10
        elif marker in "rR":
            index += 2
        elif marker in "nN":
            out.append(" ")
            index += 2
        elif marker == "|":
            out.append("|")
            index += 2
        else:
            out.append(text[index])
            index += 1
    return " ".join("".join(out).split())


def map_text(item: dict) -> tuple[str, str]:
    """Return the clean title and description supplied to the model."""
    title = strip_warcraft_codes(item.get("name") or item.get("filename"))
    description = strip_warcraft_codes(item.get("description"))
    return title, description


def seed_tags(item: dict) -> tuple[list[str], list[str]]:
    """Produce deterministic candidates and auditable evidence from clean text."""
    title, description = map_text(item)
    text = f"{title}\n{description}"
    tagged: list[str] = []
    evidence: list[str] = []
    for rule in TITLE_RULES:
        match = rule.pattern.search(text)
        if match and rule.tag not in tagged:
            tagged.append(rule.tag)
            evidence.append(f"{rule.tag}=text:{match.group(0)}")
    if item.get("content_type") == "campaign" and "玩法:战役" not in tagged:
        tagged.append("玩法:战役")
        evidence.append("玩法:战役=content_type:campaign")
    if not any(tag.startswith("玩法:") for tag in tagged):
        legacy = LEGACY_GAMEPLAY.get(str(item.get("collection") or ""))
        if legacy:
            tagged.append(legacy)
            evidence.append(f"{legacy}=legacy_collection:{item['collection']}")
        else:
            tagged.append("玩法:其他")
            evidence.append("玩法:其他=no_high_precision_signal")
    return tagged, evidence


def taxonomy_prompt() -> str:
    """The closed vocabulary and decision policy embedded in every batch."""
    return "\n".join(
        (
            "你是 Warcraft III 地图编目员。只依据清洗后的标题与简介做多标签分类。",
            "旧栏目只是可能错误的历史线索，不能把它当成事实。",
            "每张图必须保留至少一个以“玩法:”开头的标签；只有没有足够证据时才用“玩法:其他”。",
            "只能选择以下标签，绝不能创造同义词或新标签：",
            ", ".join((*GAMEPLAY_TAGS, *SERIES_TAGS, *THEME_TAGS)),
            "置信度只能是 high、medium、low。",
            "输入中每张图有 i（批内序号）。返回严格 JSON：{\"maps\":[{\"i\":0,\"tags\":[\"...\"],\"confidence\":\"high\"}]}。",
        )
    )

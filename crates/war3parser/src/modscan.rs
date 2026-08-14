//! Detection of known third-party modifications injected into map scripts.
//!
//! Repack tools add their own code to `war3map.j` and leave the same literals
//! in every map they touch, so a single map is enough to recognise one: no
//! reference copy of the unmodified map is needed.
//!
//! ```ignore
//! use war3parser::prelude::War3MapW3x;
//!
//! let mut map = War3MapW3x::open("map.w3x")?;
//! if let Some(found) = war3parser::modscan::detect(&mut map) {
//!     println!("{} — {:?}", found.label, found.activation);
//! }
//! ```

use crate::archive::War3MapW3x;

/// Candidate script paths, in the order the game resolves them.
const SCRIPT_PATHS: &[&str] = &[
    "war3map.j",
    "scripts\\war3map.j",
    "war3map.lua",
    "scripts\\war3map.lua",
];

/// A modification recognised inside a map script.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModInfo {
    /// Stable identifier of the tool.
    pub tool: String,
    /// Human-readable name.
    pub label: String,
    /// Build string taken from the injected banner, when recognised.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub variant: Option<String>,
    /// How a player triggers the injected menu, in the tool's own terms.
    pub activation: Vec<String>,
    /// Which literals matched, so a result can be audited without a rescan.
    pub evidence: Vec<String>,
    /// Where the tool documents itself.
    pub reference: String,
}

/// Literals the HKE (火龙) cheat script cannot drop: the banner doubles as the
/// menu title and as the source of its own chat phrase, assembled with
/// `SubStringBJ`, so removing it breaks the injected code.
const HKE_BANNER_MARKERS: &[(&str, &str)] = &[
    ("WuHansen", "banner:site"),
    ("wuhansen", "banner:site"),
    ("WUHANSEN", "banner:site"),
    ("21764538", "banner:qq"),
    ("19938997", "banner:qq-group"),
];

/// Known banner builds, longest first so `hke1.25B` wins over `hke1.25`.
const HKE_VARIANTS: &[&str] = &[
    "CheatEngine1.25",
    "Hke1.25B",
    "hke1.25B",
    "orz1.25B",
    "Hke1.25",
    "hke1.25",
];

fn contains(haystack: &[u8], needle: &str) -> bool {
    let pat = needle.as_bytes();
    if pat.is_empty() || haystack.len() < pat.len() {
        return false;
    }
    haystack.windows(pat.len()).any(|window| window == pat)
}

/// Read the first script present in the archive.
fn read_script(archive: &mut War3MapW3x) -> Option<Vec<u8>> {
    SCRIPT_PATHS
        .iter()
        .find_map(|path| archive.read_file(path).ok())
}

/// Inspect a map's script for known modifications.
///
/// `None` means no known signature matched — not that the map is unmodified.
/// A protected map whose script cannot be read is indistinguishable from a
/// clean one here.
pub fn detect(archive: &mut War3MapW3x) -> Option<ModInfo> {
    let script = read_script(archive)?;
    detect_in_script(&script)
}

/// Inspect raw script bytes. Split out so the rules stay unit-testable.
pub fn detect_in_script(script: &[u8]) -> Option<ModInfo> {
    let mut evidence = Vec::new();
    for (marker, tag) in HKE_BANNER_MARKERS {
        if contains(script, marker) {
            evidence.push((*tag).to_string());
            break;
        }
    }
    if evidence.is_empty() {
        return None;
    }

    // The activation steps match both the tool's own documentation
    // (https://www.wuhansen.com/warmap/) and the events the injected JASS
    // registers: four arrow-key handlers driving a per-player state machine,
    // and one non-exact chat event on "-".
    let mut activation = Vec::new();
    if contains(script, "TriggerRegisterPlayerKeyEventBJ") {
        evidence.push("arrow-key-triggers".to_string());
        activation.push("方向键依次按 ↑↑←↓ 开启作弊，之后方向键操作弹出菜单".to_string());
    }
    if contains(script, ",\"-\",false)") {
        evidence.push("chat-dash-event".to_string());
        activation.push("聊天栏输入 - 开头的命令，-h 查键盘作弊、-c 查命令列表".to_string());
    }
    if contains(script, "\"iam\"+SubStringBJ") {
        evidence.push("hidden-phrase".to_string());
        activation.push("暗语 iamWuHansen".to_string());
    }
    if contains(script, "EVENT_PLAYER_END_CINEMATIC") {
        evidence.push("esc-event".to_string());
        activation.push("Esc 用于清 CD、回血魔与切换背包".to_string());
    }

    Some(ModInfo {
        tool: "hke".to_string(),
        label: "HKE 作弊脚本（火龙）".to_string(),
        variant: HKE_VARIANTS
            .iter()
            .find(|variant| contains(script, variant))
            .map(|variant| (*variant).to_string()),
        activation,
        evidence,
        reference: "https://www.wuhansen.com/warmap/".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_script_is_not_flagged() {
        let script = b"function main takes nothing returns nothing\nendfunction";
        assert!(detect_in_script(script).is_none());
    }

    #[test]
    fn hke_banner_and_triggers_are_reported() {
        let script = concat!(
            "string hke_Z0z=\"菜单 |cFFFF0000Hke1.25B|r By 火龙 QQ:21764538",
            "|n|cFF00FF33Www.WuHansen.Com|r\"\n",
            "call TriggerRegisterPlayerKeyEventBJ(hke_z00[0],p,0,3)\n",
            "call TriggerRegisterPlayerChatEvent(hke_zz1[0],p,\"-\",false)\n",
            "if(s==\"iam\"+SubStringBJ(hke_Z0z,139,146))then\n",
        )
        .as_bytes();
        let info = detect_in_script(script).expect("should detect");
        assert_eq!(info.tool, "hke");
        assert_eq!(info.variant.as_deref(), Some("Hke1.25B"));
        assert!(info.evidence.iter().any(|e| e == "arrow-key-triggers"));
        assert!(info.evidence.iter().any(|e| e == "chat-dash-event"));
        assert!(info.evidence.iter().any(|e| e == "hidden-phrase"));
        assert_eq!(info.activation.len(), 3);
    }

    #[test]
    fn qq_number_alone_is_enough() {
        let script = b"string x=\"By Hke QQ:21764538\"";
        let info = detect_in_script(script).expect("should detect");
        assert_eq!(info.evidence.first().map(String::as_str), Some("banner:qq"));
        assert!(info.variant.is_none());
    }
}

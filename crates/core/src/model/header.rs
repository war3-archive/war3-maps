//! `HM3W` container header (absent on bare-MPQ / protected maps).

/// Fields of the optional `HM3W` prefix that precedes the embedded MPQ.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapHeader {
    /// Whether the `HM3W` magic was present.
    pub has_hm3w: bool,
    pub name: Option<String>,
    pub flags: Option<u32>,
    pub max_players: Option<u32>,
    /// Unknown field between the magic and the name.
    pub u1: Option<u32>,
}

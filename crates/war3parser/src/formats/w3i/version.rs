//! `war3map.w3i` format version ladder.

/// Format version of a `war3map.w3i` file.
///
/// Thin ordered wrapper around the raw version number so gate checks read as
/// `version >= FormatVersion::V23`. Known versions (from War3Net) are:
///
/// | Version | Game |
/// |---------|------|
/// | 8–15    | Reign of Chaos betas |
/// | 18      | Reign of Chaos |
/// | 23–25   | The Frozen Throne (25 = final TFT) |
/// | 26–27   | TFT patches |
/// | 28      | 1.31 (adds Lua script mode) |
/// | 31      | Reforged 1.32 |
/// | 32–33   | WC3 2.0 (camera zoom limits) |
///
/// Unknown numbers order naturally between / beyond the known ones, so maps
/// produced by future patches keep parsing with the nearest layout.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormatVersion(pub u32);

impl FormatVersion {
    pub const V8: Self = Self(8);
    pub const V10: Self = Self(10);
    pub const V11: Self = Self(11);
    pub const V15: Self = Self(15);
    pub const V18: Self = Self(18);
    pub const V23: Self = Self(23);
    pub const V24: Self = Self(24);
    pub const V25: Self = Self(25);
    pub const V26: Self = Self(26);
    pub const V27: Self = Self(27);
    pub const V28: Self = Self(28);
    pub const V31: Self = Self(31);
    pub const V32: Self = Self(32);
    pub const V33: Self = Self(33);

    /// All versions with a documented layout, ascending.
    pub const KNOWN: &'static [Self] = &[
        Self::V8,
        Self::V10,
        Self::V11,
        Self::V15,
        Self::V18,
        Self::V23,
        Self::V24,
        Self::V25,
        Self::V26,
        Self::V27,
        Self::V28,
        Self::V31,
        Self::V32,
        Self::V33,
    ];

    /// Whether this exact version number has a documented layout.
    pub fn is_known(self) -> bool {
        Self::KNOWN.contains(&self)
    }

    /// The Frozen Throne data layout or later.
    pub fn is_tft(self) -> bool {
        self >= Self::V23
    }

    /// Reforged (1.32+) layout or later.
    pub fn is_reforged(self) -> bool {
        self >= Self::V31
    }
}

impl From<u32> for FormatVersion {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_follows_numbers() {
        assert!(FormatVersion::V18 < FormatVersion::V25);
        assert!(FormatVersion(29) > FormatVersion::V28);
        assert!(FormatVersion(29) < FormatVersion::V31);
    }

    #[test]
    fn known_and_era_helpers() {
        assert!(FormatVersion::V25.is_known());
        assert!(!FormatVersion(29).is_known());
        assert!(FormatVersion::V25.is_tft());
        assert!(!FormatVersion::V18.is_tft());
        assert!(FormatVersion::V31.is_reforged());
        assert!(!FormatVersion::V28.is_reforged());
    }
}

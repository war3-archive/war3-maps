//! Upgrade / tech availability change records.

use crate::error::Result;
use crate::reader::ByteReader;

/// One modified upgrade availability entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct UpgradeAvailabilityChange {
    /// Bitmask of affected player slots.
    pub player_flags: u32,
    /// Four-character upgrade rawcode (e.g. `Rhme`).
    pub id: [u8; 4],
    pub level_affected: i32,
    /// 0 = unavailable, 1 = available, 2 = researched.
    pub availability: i32,
}

/// One removed (unavailable) tech entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TechAvailabilityChange {
    /// Bitmask of affected player slots.
    pub player_flags: u32,
    /// Four-character tech rawcode.
    pub id: [u8; 4],
}

impl UpgradeAvailabilityChange {
    pub(crate) fn parse(r: &mut ByteReader<'_>) -> Result<Self> {
        Ok(Self {
            player_flags: r.u32()?,
            id: r.bytes()?,
            level_affected: r.i32()?,
            availability: r.i32()?,
        })
    }
}

impl TechAvailabilityChange {
    pub(crate) fn parse(r: &mut ByteReader<'_>) -> Result<Self> {
        Ok(Self {
            player_flags: r.u32()?,
            id: r.bytes()?,
        })
    }
}

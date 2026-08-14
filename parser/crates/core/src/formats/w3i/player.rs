//! Player slot record.

use crate::error::Result;
use crate::formats::w3i::FormatVersion;
use crate::reader::ByteReader;

/// A player slot definition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Player {
    pub id: i32,
    /// Controller: 1 = user, 2 = computer, 3 = neutral, 4 = rescuable.
    pub player_type: i32,
    /// Race: 1 = human, 2 = orc, 3 = undead, 4 = night elf.
    pub race: i32,
    /// Non-zero when the start position is fixed.
    pub is_fixed_start_position: i32,
    pub name: String,
    pub start_location: [f32; 2],
    pub ally_low_priorities: u32,
    pub ally_high_priorities: u32,
    /// Enemy low-priority bitmask (v31+).
    pub enemy_low_priorities: Option<u32>,
    /// Enemy high-priority bitmask (v31+).
    pub enemy_high_priorities: Option<u32>,
}

impl Player {
    pub(crate) fn parse(r: &mut ByteReader<'_>, version: FormatVersion) -> Result<Self> {
        Ok(Self {
            id: r.i32()?,
            player_type: r.i32()?,
            race: r.i32()?,
            is_fixed_start_position: r.i32()?,
            name: r.cstr_lossy()?,
            start_location: r.f32s()?,
            ally_low_priorities: r.u32()?,
            ally_high_priorities: r.u32()?,
            enemy_low_priorities: (version >= FormatVersion::V31)
                .then(|| r.u32())
                .transpose()?,
            enemy_high_priorities: (version >= FormatVersion::V31)
                .then(|| r.u32())
                .transpose()?,
        })
    }
}

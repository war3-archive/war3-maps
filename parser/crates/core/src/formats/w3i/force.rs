//! Force (team) record.

use crate::error::Result;
use crate::reader::ByteReader;

/// A force (team) definition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Force {
    /// Bit 0: allied, bit 1: allied victory, bit 3: share vision,
    /// bit 4: share unit control, bit 5: share advanced unit control.
    pub flags: u32,
    /// Bitmask of player slots belonging to this force.
    pub player_masks: u32,
    pub name: String,
}

impl Force {
    pub(crate) fn parse(r: &mut ByteReader<'_>) -> Result<Self> {
        Ok(Self {
            flags: r.u32()?,
            player_masks: r.u32()?,
            name: r.cstr_lossy()?,
        })
    }
}

use binary_reader::BinaryReader;

use crate::parser::binary_reader::{AutoReadable, BinaryReadable};
use crate::parser::error::ParserError;

#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Player {
    pub id: i32,
    pub player_type: i32,
    pub race: i32,
    pub is_fixed_start_position: i32,
    pub name: String,
    pub start_location: [f32; 2],
    pub ally_low_priorities: u32,
    pub ally_high_priorities: u32,
    /// Enemy low-priority bitmask (`version > 30`)
    pub enemy_low_priorities: Option<u32>,
    /// Enemy high-priority bitmask (`version > 30`)
    pub enemy_high_priorities: Option<u32>,
}

impl BinaryReadable for Player {
    fn load(stream: &mut BinaryReader, version: u32) -> Result<Self, ParserError> {
        Ok(Self {
            id: AutoReadable::read(stream)?,
            player_type: AutoReadable::read(stream)?,
            race: AutoReadable::read(stream)?,
            is_fixed_start_position: AutoReadable::read(stream)?,
            name: AutoReadable::read(stream)?,
            start_location: AutoReadable::read(stream)?,
            ally_low_priorities: AutoReadable::read(stream)?,
            ally_high_priorities: AutoReadable::read(stream)?,
            enemy_low_priorities: if version > 30 {
                Some(AutoReadable::read(stream)?)
            } else {
                None
            },
            enemy_high_priorities: if version > 30 {
                Some(AutoReadable::read(stream)?)
            } else {
                None
            },
        })
    }

    fn size(&self, version: u32) -> usize {
        let mut size = 32 + self.name.len() + 1;
        if version > 30 {
            size += 8;
        }
        size
    }
}

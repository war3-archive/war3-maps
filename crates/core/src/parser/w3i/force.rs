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
pub struct Force {
    pub flags: u32,
    pub player_masks: u32,
    pub name: String,
}

impl BinaryReadable for Force {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        Ok(Self {
            flags: AutoReadable::read(stream)?,
            player_masks: AutoReadable::read(stream)?,
            name: AutoReadable::read(stream)?,
        })
    }
}

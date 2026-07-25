use binary_reader::BinaryReader;

use crate::parser::{
    binary_reader::{AutoReadable, BinaryReadable},
    error::ParserError,
};

#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomUnit {
    pub chance: i32,
    pub ids: Vec<[u8; 4]>,
}

#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomUnitTable {
    pub id: i32,
    pub name: String,
    /// Number of columns (formerly misused as unit-row count)
    pub columns: i32,
    pub column_types: Vec<i32>,
    pub units: Vec<RandomUnit>,
}

impl BinaryReadable for RandomUnit {
    fn load(stream: &mut BinaryReader, columns: u32) -> Result<Self, ParserError> {
        Ok(Self {
            chance: AutoReadable::read(stream)?,
            ids: {
                let mut ids: Vec<[u8; 4]> = Vec::with_capacity(columns as usize);
                for _ in 0..columns {
                    ids.push(AutoReadable::read(stream)?);
                }
                ids
            },
        })
    }
}

impl BinaryReadable for RandomUnitTable {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let id: i32 = AutoReadable::read(stream)?;
        let name: String = AutoReadable::read(stream)?;
        let columns: i32 = AutoReadable::read(stream)?;
        let mut column_types: Vec<i32> = Vec::with_capacity(columns as usize);
        for _ in 0..columns {
            column_types.push(AutoReadable::read(stream)?);
        }
        // Separate row count — not the same as `columns`
        let rows: u32 = AutoReadable::read(stream)?;
        let mut units: Vec<RandomUnit> = Vec::with_capacity(rows as usize);
        for _ in 0..rows {
            units.push(RandomUnit::load(stream, columns as u32)?);
        }
        Ok(Self {
            id,
            name,
            columns,
            column_types,
            units,
        })
    }
}

//! Random unit/item table records.

use crate::error::Result;
use crate::reader::{parse_counted, ByteReader};

/// One row of a random unit table: a chance and one rawcode per column.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomUnit {
    pub chance: i32,
    pub ids: Vec<[u8; 4]>,
}

/// A random unit table (`Set Random Group` in the editor).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomUnitTable {
    pub id: i32,
    pub name: String,
    /// Number of columns.
    pub columns: i32,
    /// Per-column widget type: 0 = unit, 1 = building, 2 = item.
    pub column_types: Vec<i32>,
    /// Rows; each row has `columns` rawcodes.
    pub units: Vec<RandomUnit>,
}

/// One random item entry: a chance and an item rawcode.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomItem {
    pub chance: i32,
    pub id: [u8; 4],
}

/// A set of random items rolled together.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomItemSet {
    pub items: Vec<RandomItem>,
}

/// A random item table (`Set Random Item Group` in the editor).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RandomItemTable {
    pub id: i32,
    pub name: String,
    pub sets: Vec<RandomItemSet>,
}

impl RandomUnitTable {
    pub(crate) fn parse(r: &mut ByteReader<'_>) -> Result<Self> {
        let id = r.i32()?;
        let name = r.cstr_lossy()?;
        let columns = r.i32()?;
        let mut column_types = Vec::with_capacity(columns.max(0) as usize);
        for _ in 0..columns {
            column_types.push(r.i32()?);
        }
        let rows = r.u32()?;
        let mut units = Vec::with_capacity((rows as usize).min(4096));
        for _ in 0..rows {
            let chance = r.i32()?;
            let mut ids = Vec::with_capacity(columns.max(0) as usize);
            for _ in 0..columns {
                ids.push(r.bytes()?);
            }
            units.push(RandomUnit { chance, ids });
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

impl RandomItemTable {
    pub(crate) fn parse(r: &mut ByteReader<'_>) -> Result<Self> {
        Ok(Self {
            id: r.i32()?,
            name: r.cstr_lossy()?,
            sets: parse_counted(r, |r| {
                Ok(RandomItemSet {
                    items: parse_counted(r, |r| {
                        Ok(RandomItem {
                            chance: r.i32()?,
                            id: r.bytes()?,
                        })
                    })?,
                })
            })?,
        })
    }
}

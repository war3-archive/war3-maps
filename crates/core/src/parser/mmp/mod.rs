//! `war3map.mmp` — minimap icon overlay (gold mines, houses, player starts)

use binary_reader::BinaryReader;

use super::{
    binary_reader::{AutoReadable, BinaryReadable},
    error::ParserError,
};

/// Icon type stored in `war3map.mmp`
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapIconType {
    GoldMine = 0,
    House = 1,
    PlayerStart = 2,
    Unknown = -1,
}

impl From<i32> for MinimapIconType {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::GoldMine,
            1 => Self::House,
            2 => Self::PlayerStart,
            _ => Self::Unknown,
        }
    }
}

/// Single minimap icon entry
#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct MinimapIcon {
    /// 0 = gold mine, 1 = house/neutral building, 2 = player start
    pub icon_type: i32,
    /// Pixel coordinates on the 256×256 minimap texture (origin top-left)
    pub x: i32,
    pub y: i32,
    /// Color as RGBA (converted from file BGRA)
    pub color: [u8; 4],
}

impl MinimapIcon {
    pub fn kind(&self) -> MinimapIconType {
        MinimapIconType::from(self.icon_type)
    }
}

/// Minimap icon table (`war3map.mmp`)
#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapMmp {
    pub version: i32,
    pub icons: Vec<MinimapIcon>,
}

impl BinaryReadable for MinimapIcon {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let icon_type: i32 = AutoReadable::read(stream)?;
        let x: i32 = AutoReadable::read(stream)?;
        let y: i32 = AutoReadable::read(stream)?;
        // File stores BGRA
        let b: u8 = AutoReadable::read(stream)?;
        let g: u8 = AutoReadable::read(stream)?;
        let r: u8 = AutoReadable::read(stream)?;
        let a: u8 = AutoReadable::read(stream)?;
        Ok(Self {
            icon_type,
            x,
            y,
            color: [r, g, b, a],
        })
    }
}

impl BinaryReadable for War3MapMmp {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let version: i32 = AutoReadable::read(stream)?;
        let count: u32 = AutoReadable::read(stream)?;
        let mut icons = Vec::with_capacity(count as usize);
        for _ in 0..count {
            icons.push(MinimapIcon::load(stream, 0)?);
        }
        Ok(Self { version, icons })
    }
}

impl War3MapMmp {
    pub fn load_bytes(data: &[u8]) -> Result<Self, ParserError> {
        let mut reader = BinaryReader::from_u8(data);
        reader.set_endian(binary_reader::Endian::Little);
        Self::load(&mut reader, 0)
    }
}

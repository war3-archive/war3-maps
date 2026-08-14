//! `war3map.mmp` — minimap icon overlay (gold mines, houses, player starts).

use crate::error::Result;
use crate::reader::{parse_counted, ByteReader};

/// Icon type stored in `war3map.mmp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapIconType {
    GoldMine,
    House,
    PlayerStart,
    Unknown,
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

/// Single minimap icon entry.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct MinimapIcon {
    /// 0 = gold mine, 1 = house/neutral building, 2 = player start.
    pub icon_type: i32,
    /// Pixel X on the 256×256 minimap texture (origin top-left).
    pub x: i32,
    /// Pixel Y on the 256×256 minimap texture (origin top-left).
    pub y: i32,
    /// Color as RGBA (converted from the file's BGRA order).
    pub color: [u8; 4],
}

impl MinimapIcon {
    /// Typed view of `icon_type`.
    pub fn kind(&self) -> MinimapIconType {
        MinimapIconType::from(self.icon_type)
    }
}

/// Minimap icon table parsed from `war3map.mmp`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapMmp {
    pub version: i32,
    pub icons: Vec<MinimapIcon>,
}

impl War3MapMmp {
    /// Parse a complete `war3map.mmp` buffer.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let r = &mut ByteReader::new(data);
        Ok(Self {
            version: r.i32()?,
            icons: parse_counted(r, |r| {
                let icon_type = r.i32()?;
                let x = r.i32()?;
                let y = r.i32()?;
                let [b, g, red, a] = r.bytes()?;
                Ok(MinimapIcon {
                    icon_type,
                    x,
                    y,
                    color: [red, g, b, a],
                })
            })?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_icons_and_swaps_bgra() {
        let mut data = Vec::new();
        data.extend(0i32.to_le_bytes()); // version
        data.extend(1u32.to_le_bytes()); // count
        data.extend(2i32.to_le_bytes()); // player start
        data.extend(100i32.to_le_bytes());
        data.extend(200i32.to_le_bytes());
        data.extend([0x10, 0x20, 0x30, 0x40]); // BGRA

        let mmp = War3MapMmp::parse(&data).unwrap();
        assert_eq!(mmp.icons.len(), 1);
        let icon = &mmp.icons[0];
        assert_eq!(icon.kind(), MinimapIconType::PlayerStart);
        assert_eq!((icon.x, icon.y), (100, 200));
        assert_eq!(icon.color, [0x30, 0x20, 0x10, 0x40]); // RGBA
    }
}

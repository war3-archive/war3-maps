use war3parser::parser::mmp::MinimapIcon as MinimapIconOri;
use war3parser::parser::w3i::War3MapW3i;
use war3parser::prelude::{War3Image as War3ImageOri, War3ImageBase64};
use war3parser::war3map_metadata::War3MapHeader;

/// Preview and minimap images
#[derive(Debug, Clone, tsify_next::Tsify, serde::Serialize, serde::Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct War3Image {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub filename: String,
}

impl TryFrom<&War3ImageOri> for War3Image {
    type Error = ();

    fn try_from(image: &War3ImageOri) -> Result<Self, Self::Error> {
        let width = image.data.width();
        let height = image.data.height();
        let war3image_base64 = War3ImageBase64::try_from((*image).clone()).map_err(|_| ())?;
        Ok(Self {
            data_url: war3image_base64.data,
            width,
            height,
            filename: war3image_base64.filename,
        })
    }
}

#[derive(Debug, Clone, tsify_next::Tsify, serde::Serialize, serde::Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct StringTableEntry {
    pub id: i32,
    pub value: String,
}

#[derive(Debug, Clone, tsify_next::Tsify, serde::Serialize, serde::Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ImportEntry {
    pub path: String,
    pub is_custom: u8,
}

/// Minimap overlay icon (from `war3map.mmp`)
///
/// Coordinates are pixels on the canonical 256×256 minimap.
/// `icon_type`: 0 gold mine, 1 house, 2 player start.
#[derive(Debug, Clone, tsify_next::Tsify, serde::Serialize, serde::Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct MinimapIcon {
    pub icon_type: i32,
    pub x: i32,
    pub y: i32,
    /// RGBA
    pub color: [u8; 4],
}

impl From<&MinimapIconOri> for MinimapIcon {
    fn from(icon: &MinimapIconOri) -> Self {
        Self {
            icon_type: icon.icon_type,
            x: icon.x,
            y: icon.y,
            color: icon.color,
        }
    }
}

/// Full map metadata exposed to JavaScript
#[derive(Debug, Clone, tsify_next::Tsify, serde::Serialize, serde::Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct War3MapMetadata {
    pub header: War3MapHeader,
    pub map_info: Option<War3MapW3i>,
    pub images: Vec<War3Image>,
    pub minimap_icons: Vec<MinimapIcon>,
    pub imports: Option<Vec<ImportEntry>>,
    pub strings: Option<Vec<StringTableEntry>>,
    pub files: Option<Vec<String>>,
    /// Wall-clock milliseconds spent parsing
    pub parse_ms: f64,
}

//! High-level map metadata and the portable [`MapSnapshot`] API surface.

use std::path::Path;

use crate::model::{
    header::War3MapHeader, image::War3Image, image::War3ImageData, import::ImportEntry,
    string_table::StringTableEntry,
};
use crate::parser::{
    error::ParserError, imp::War3MapImp, mmp::MinimapIcon, mmp::War3MapMmp, w3i::War3MapW3i,
    w3x::War3MapW3x, wts::War3MapWts,
};

/// Rich in-memory parse result.
///
/// Holds non-serializable assets such as [`War3Image`] rasters. Convert to
/// [`MapSnapshot`] for CLI dumps, JSON, or WASM.
pub struct War3MapMetadata {
    pub header: War3MapHeader,
    pub map_info: Option<War3MapW3i>,
    pub imp: Option<War3MapImp>,
    pub wts: Option<War3MapWts>,
    pub images: Vec<War3Image>,
    /// Minimap icons from `war3map.mmp` (gold mines, houses, starts).
    pub minimap_icons: Option<War3MapMmp>,
    pub files: Option<Vec<String>>,
}

/// Portable map snapshot shared by the CLI, WASM bindings, and serde dumps.
///
/// This is the canonical cross-crate API shape — prefer it over inventing
/// parallel DTOs in `war3parser-cli` / `war3parser-wasm`.
#[cfg_attr(
    feature = "typescript",
    derive(tsify_next::Tsify),
    tsify(into_wasm_abi, from_wasm_abi)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct MapSnapshot {
    pub header: War3MapHeader,
    pub map_info: Option<War3MapW3i>,
    pub images: Vec<War3ImageData>,
    pub minimap_icons: Vec<MinimapIcon>,
    pub imports: Option<Vec<ImportEntry>>,
    pub strings: Option<Vec<StringTableEntry>>,
    pub files: Option<Vec<String>>,
    /// Wall-clock milliseconds spent parsing (filled by WASM; usually `None` natively).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub parse_ms: Option<f64>,
}

impl War3MapMetadata {
    /// Load metadata from a buffer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use war3parser::prelude::War3MapMetadata;
    ///
    /// let buffer = std::fs::read("path/to/map.w3x").unwrap();
    /// let metadata = War3MapMetadata::from(&buffer).unwrap();
    /// ```
    pub fn from(buffer: &[u8]) -> Option<Self> {
        let w3x = War3MapW3x::from_buffer(buffer).ok()?;
        let mut w3x = Box::new(w3x);
        let mut images = Vec::new();
        if let Ok(minimap) = w3x.read_minimap() {
            images.push(minimap);
        }
        if let Ok(preview) = w3x.read_preview() {
            images.push(preview);
        }

        let mut files = w3x.files.clone();
        if let Some(ref mut list) = files {
            list.sort();
        }

        Some(Self {
            header: w3x.header.clone(),
            map_info: w3x.read_map_info().ok(),
            imp: w3x.read_imports().ok(),
            wts: w3x.read_string_table().ok(),
            images,
            minimap_icons: w3x.read_minimap_icons().ok(),
            files,
        })
    }

    /// Parse a buffer and immediately produce a portable [`MapSnapshot`].
    ///
    /// Applies TRIGSTR resolution when a string table is present.
    pub fn parse_snapshot(buffer: &[u8]) -> Option<MapSnapshot> {
        let mut metadata = Self::from(buffer)?;
        let _ = metadata.update_string_table();
        metadata.snapshot().ok()
    }

    /// Update trigger strings in `w3i` when a `wts` table is available.
    pub fn update_string_table(&mut self) -> Result<(), ParserError> {
        #[cfg(feature = "serde")]
        {
            let map_info = self
                .map_info
                .as_ref()
                .ok_or_else(|| ParserError::MapFileNotFound("w3i".to_string()))?;
            let mut map_info_json = serde_json::to_string_pretty(map_info)?;
            let trigger_string_map = map_info.trigger_string_map()?;

            let string_table = &self
                .wts
                .as_ref()
                .ok_or_else(|| ParserError::MapFileNotFound("wts".to_string()))?
                .string_map;

            let default = "Unknown".to_string();

            for (key, value) in &trigger_string_map {
                let replace_str = string_table.get(value).unwrap_or(&default);
                let replace_str = serde_json::to_string(replace_str)?;
                map_info_json = map_info_json.replace(key, replace_str.as_str());
            }

            self.map_info = Some(serde_json::from_str(&map_info_json)?);
            Ok(())
        }

        #[cfg(not(feature = "serde"))]
        {
            Err(ParserError::FeatureRequired("serde"))
        }
    }

    /// Build a portable snapshot from the rich in-memory metadata.
    pub fn snapshot(&self) -> Result<MapSnapshot, ParserError> {
        let images = self
            .images
            .iter()
            .map(War3ImageData::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let minimap_icons: Vec<MinimapIcon> = self
            .minimap_icons
            .as_ref()
            .map(|m| m.icons.clone())
            .unwrap_or_default();

        let imports = self.imp.as_ref().map(War3MapImp::entries_sorted);
        let strings = self.wts.as_ref().map(War3MapWts::entries_sorted);

        Ok(MapSnapshot {
            header: self.header.clone(),
            map_info: self.map_info.clone(),
            images,
            minimap_icons,
            imports,
            strings,
            files: self.files.clone(),
            parse_ms: None,
        })
    }

    /// Save metadata (JSON + PNG images) under `out_dir`.
    pub fn save(&self, out_dir: &str) -> Result<(), ParserError> {
        let snapshot = self.snapshot()?;
        snapshot.save(out_dir)?;

        // Also write raw raster PNGs (without base64) for CLI convenience.
        let out_dir = Path::new(out_dir);
        for (index, image) in self.images.iter().enumerate() {
            let image_path = out_dir.join(format!("{}_{}.png", image.filename, index));
            image.data.save(image_path)?;
        }
        Ok(())
    }
}

impl MapSnapshot {
    /// Serialize this snapshot as pretty JSON files under `out_dir`.
    pub fn save(&self, out_dir: &str) -> Result<(), ParserError> {
        let out_dir = Path::new(out_dir);
        std::fs::create_dir_all(out_dir)?;

        #[cfg(feature = "serde")]
        {
            std::fs::write(
                out_dir.join("header.json"),
                serde_json::to_string_pretty(&self.header)?,
            )?;

            if let Some(map_info) = &self.map_info {
                std::fs::write(
                    out_dir.join("war3map.w3i.json"),
                    serde_json::to_string_pretty(map_info)?,
                )?;
            }
            if let Some(strings) = &self.strings {
                std::fs::write(
                    out_dir.join("war3map.wts.json"),
                    serde_json::to_string_pretty(strings)?,
                )?;
            }
            if let Some(imports) = &self.imports {
                std::fs::write(
                    out_dir.join("war3map.imp.json"),
                    serde_json::to_string_pretty(imports)?,
                )?;
            }
            if let Some(files) = &self.files {
                std::fs::write(
                    out_dir.join("listfile.json"),
                    serde_json::to_string_pretty(files)?,
                )?;
            }
            if !self.minimap_icons.is_empty() {
                std::fs::write(
                    out_dir.join("war3map.mmp.json"),
                    serde_json::to_string_pretty(&self.minimap_icons)?,
                )?;
            }
            if !self.images.is_empty() {
                std::fs::write(
                    out_dir.join("images.json"),
                    serde_json::to_string_pretty(&self.images)?,
                )?;
            }
            Ok(())
        }

        #[cfg(not(feature = "serde"))]
        {
            let _ = out_dir;
            Err(ParserError::FeatureRequired("serde"))
        }
    }
}

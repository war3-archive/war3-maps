//! High-level orchestration: parse everything a map contains in one call.

use std::path::Path;

use crate::archive::War3MapW3x;
use crate::error::Result;
use crate::formats::imp::War3MapImp;
use crate::formats::mmp::{MinimapIcon, War3MapMmp};
use crate::formats::w3i::War3MapW3i;
use crate::formats::wts::{trigstr_id, War3MapWts};
use crate::model::{
    header::War3MapHeader, image::War3Image, image::War3ImageData, snapshot::MapSnapshot,
};
use crate::modscan::{self, ModInfo};

/// Placeholder for TRIGSTR references that are missing from the string table.
const UNRESOLVED_TRIGSTR: &str = "Unknown";

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
    /// Third-party modification detected in the map script, if any.
    pub modification: Option<ModInfo>,
}

impl War3MapMetadata {
    /// Parse a map buffer, reading every known member file.
    ///
    /// Individual member files that are missing or corrupt simply come back
    /// as `None` / empty — only an unreadable archive is an error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use war3parser::prelude::War3MapMetadata;
    ///
    /// let buffer = std::fs::read("path/to/map.w3x")?;
    /// let mut metadata = War3MapMetadata::parse(&buffer)?;
    /// metadata.resolve_trigger_strings();
    /// let snapshot = metadata.snapshot()?;
    /// ```
    pub fn parse(buffer: &[u8]) -> Result<Self> {
        let mut w3x = War3MapW3x::from_buffer(buffer)?;

        let mut images = Vec::new();
        if let Ok(minimap) = w3x.read_minimap() {
            images.push(minimap);
        }
        if let Ok(preview) = w3x.read_preview() {
            images.push(preview);
        }

        let mut files = w3x.files.clone();
        if let Some(list) = &mut files {
            list.sort();
        }

        Ok(Self {
            header: w3x.header.clone(),
            map_info: w3x.read_map_info().ok(),
            imp: w3x.read_imports().ok(),
            wts: w3x.read_string_table().ok(),
            images,
            minimap_icons: w3x.read_minimap_icons().ok(),
            files,
            modification: modscan::detect(&mut w3x),
        })
    }

    /// Parse a buffer and immediately produce a portable [`MapSnapshot`],
    /// with TRIGSTR references resolved when a string table is present.
    pub fn parse_snapshot(buffer: &[u8]) -> Result<MapSnapshot> {
        let mut metadata = Self::parse(buffer)?;
        metadata.resolve_trigger_strings();
        metadata.snapshot()
    }

    /// Replace `TRIGSTR_<id>` references in the map info with their values
    /// from the string table. No-op when either part is missing.
    pub fn resolve_trigger_strings(&mut self) {
        let (Some(map_info), Some(wts)) = (&mut self.map_info, &self.wts) else {
            return;
        };
        map_info.visit_strings(|s| {
            if let Some(id) = trigstr_id(s) {
                *s = wts.get(id).unwrap_or(UNRESOLVED_TRIGSTR).to_string();
            }
        });
    }

    /// Build a portable snapshot from the rich in-memory metadata.
    pub fn snapshot(&self) -> Result<MapSnapshot> {
        let images = self
            .images
            .iter()
            .map(War3ImageData::try_from)
            .collect::<Result<Vec<_>>>()?;

        let minimap_icons: Vec<MinimapIcon> = self
            .minimap_icons
            .as_ref()
            .map(|m| m.icons.clone())
            .unwrap_or_default();

        Ok(MapSnapshot {
            header: self.header.clone(),
            map_info: self.map_info.clone(),
            images,
            minimap_icons,
            imports: self.imp.as_ref().map(War3MapImp::entries_sorted),
            strings: self.wts.as_ref().map(War3MapWts::entries_sorted),
            files: self.files.clone(),
            modification: self.modification.clone(),
            parse_ms: None,
        })
    }

    /// Save metadata (JSON + PNG images) under `out_dir`.
    pub fn save(&self, out_dir: impl AsRef<Path>) -> Result<()> {
        let out_dir = out_dir.as_ref();
        self.snapshot()?.save(out_dir)?;

        // Also write raw raster PNGs (without base64) for CLI convenience.
        for (index, image) in self.images.iter().enumerate() {
            let image_path = out_dir.join(format!("{}_{}.png", image.filename, index));
            image.data.save(image_path)?;
        }
        Ok(())
    }
}

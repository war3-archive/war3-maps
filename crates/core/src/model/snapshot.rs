//! Portable [`MapSnapshot`] — the canonical cross-crate API shape.

use std::path::Path;

use crate::error::Result;
use crate::formats::imp::War3MapImp;
use crate::formats::mmp::MinimapIcon;
use crate::formats::w3i::War3MapW3i;
use crate::formats::wts::War3MapWts;
use crate::model::{header::War3MapHeader, image::War3ImageData};

/// Single import path as exposed to CLI/WASM consumers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// Resolved in-archive path (standard imports carry the
    /// `war3mapimported\` prefix).
    pub path: String,
    /// Flag byte: WC3MapSpec uses 8 = standard, 13 = custom;
    /// older tools write 0/1 or 10.
    pub is_custom: u8,
}

/// Single WTS entry as exposed to CLI/WASM consumers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct StringTableEntry {
    pub id: i32,
    pub value: String,
}

/// Portable map snapshot shared by the CLI, WASM bindings, and serde dumps.
///
/// This is the canonical cross-crate API shape — prefer it over inventing
/// parallel DTOs in downstream crates.
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
    /// Wall-clock milliseconds spent parsing (filled by WASM; `None` natively).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub parse_ms: Option<f64>,
}

impl War3MapImp {
    /// Flatten entries into a path-sorted list with resolved paths.
    pub fn entries_sorted(&self) -> Vec<ImportEntry> {
        let mut entries: Vec<ImportEntry> = self
            .entries
            .iter()
            .map(|import| ImportEntry {
                path: import.resolved_path(),
                is_custom: import.is_custom,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries.dedup_by(|a, b| a.path == b.path);
        entries
    }
}

impl War3MapWts {
    /// Flatten the string map into an id-sorted list.
    pub fn entries_sorted(&self) -> Vec<StringTableEntry> {
        let mut entries: Vec<StringTableEntry> = self
            .string_map
            .iter()
            .map(|(&id, value)| StringTableEntry {
                id,
                value: value.clone(),
            })
            .collect();
        entries.sort_by_key(|e| e.id);
        entries
    }
}

impl MapSnapshot {
    /// Serialize this snapshot as pretty JSON files under `out_dir`.
    ///
    /// Requires the `serde` feature.
    pub fn save(&self, out_dir: impl AsRef<Path>) -> Result<()> {
        #[cfg(feature = "serde")]
        {
            let out_dir = out_dir.as_ref();
            std::fs::create_dir_all(out_dir)?;

            fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
                std::fs::write(path, serde_json::to_string_pretty(value)?)?;
                Ok(())
            }

            write_json(&out_dir.join("header.json"), &self.header)?;
            if let Some(map_info) = &self.map_info {
                write_json(&out_dir.join("war3map.w3i.json"), map_info)?;
            }
            if let Some(strings) = &self.strings {
                write_json(&out_dir.join("war3map.wts.json"), strings)?;
            }
            if let Some(imports) = &self.imports {
                write_json(&out_dir.join("war3map.imp.json"), imports)?;
            }
            if let Some(files) = &self.files {
                write_json(&out_dir.join("listfile.json"), files)?;
            }
            if !self.minimap_icons.is_empty() {
                write_json(&out_dir.join("war3map.mmp.json"), &self.minimap_icons)?;
            }
            if !self.images.is_empty() {
                write_json(&out_dir.join("images.json"), &self.images)?;
            }
            Ok(())
        }

        #[cfg(not(feature = "serde"))]
        {
            let _ = out_dir;
            Err(crate::error::Error::FeatureRequired("serde"))
        }
    }
}

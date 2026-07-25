use war3parser::war3map_metadata::War3MapMetadata as War3MapMetadataOri;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ImportEntry, MinimapIcon, StringTableEntry, War3Image, War3MapMetadata};

fn build_metadata(buffer: &[u8]) -> Option<War3MapMetadata> {
    let started = js_sys::Date::now();
    let mut metadata = War3MapMetadataOri::from(buffer)?;
    // Non-fatal if WTS is missing
    let _ = metadata.update_string_table();

    let map_info = metadata.map_info.take();
    let images: Vec<War3Image> = metadata
        .images
        .iter()
        .filter_map(|img| War3Image::try_from(img).ok())
        .collect();

    let minimap_icons: Vec<MinimapIcon> = metadata
        .minimap_icons
        .as_ref()
        .map(|m| m.icons.iter().map(MinimapIcon::from).collect())
        .unwrap_or_default();

    let imports = metadata.imp.map(|imp| {
        let mut entries: Vec<ImportEntry> = imp
            .entries
            .into_iter()
            .map(|(path, entry)| ImportEntry {
                path,
                is_custom: entry.is_custom,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    });

    let strings = metadata.wts.map(|wts| {
        let mut entries: Vec<StringTableEntry> = wts
            .string_map
            .into_iter()
            .map(|(id, value)| StringTableEntry { id, value })
            .collect();
        entries.sort_by_key(|e| e.id);
        entries
    });

    let mut files = metadata.files;
    if let Some(ref mut list) = files {
        list.sort();
    }

    Some(War3MapMetadata {
        header: metadata.header,
        map_info,
        images,
        minimap_icons,
        imports,
        strings,
        files,
        parse_ms: js_sys::Date::now() - started,
    })
}

/// Parse a full War3 map (`.w3x` / `.w3m`) buffer.
///
/// Returns `None` if the buffer is not a readable MPQ/map.
#[wasm_bindgen]
pub fn parse_map(buffer: js_sys::Uint8Array) -> Option<War3MapMetadata> {
    build_metadata(&buffer.to_vec())
}

/// Backward-compatible alias of [`parse_map`].
#[wasm_bindgen]
pub fn get_map_info(buffer: js_sys::Uint8Array) -> Option<War3MapMetadata> {
    parse_map(buffer)
}

/// Crate version string for UI badges
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

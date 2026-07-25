//! WASM bindings for war3parser.
//!
//! Thin glue over [`war3parser::model::MapSnapshot`]. All shared data structures
//! live in the core crate — this crate only exposes `wasm_bindgen` entry points.
//!
//! [![NPM Version](https://img.shields.io/npm/v/%40wesleyel%2Fwar3parser)](https://www.npmjs.com/package/@wesleyel/war3parser)
//!
//! ## Example
//!
//! ```ignore
//! import init, { parse_map, version } from "@wesleyel/war3parser";
//!
//! await init();
//! const meta = parse_map(new Uint8Array(buffer));
//! console.log(version(), meta?.map_info?.name);
//! ```

use war3parser::prelude::{MapSnapshot, War3MapMetadata};
use wasm_bindgen::prelude::wasm_bindgen;

/// Parse a full War3 map (`.w3x` / `.w3m`) buffer.
///
/// Returns `None` if the buffer is not a readable MPQ/map.
///
/// The returned value is core's [`MapSnapshot`] (images as PNG data URLs,
/// sorted imports/strings, optional `parse_ms`).
#[wasm_bindgen]
pub fn parse_map(buffer: js_sys::Uint8Array) -> Option<MapSnapshot> {
    let started = js_sys::Date::now();
    let mut snapshot = War3MapMetadata::parse_snapshot(&buffer.to_vec())?;
    snapshot.parse_ms = Some(js_sys::Date::now() - started);
    Some(snapshot)
}

/// Backward-compatible alias of [`parse_map`].
#[wasm_bindgen]
pub fn get_map_info(buffer: js_sys::Uint8Array) -> Option<MapSnapshot> {
    parse_map(buffer)
}

/// Crate version string for UI badges.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

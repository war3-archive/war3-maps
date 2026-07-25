//! WASM bindings for war3parser.
//!
//! Thin glue over [`war3parser::model::MapSnapshot`]. Core stays free of
//! wasm-bindgen; values cross the boundary via `serde-wasm-bindgen`.
//!
//! TypeScript definitions are maintained in `war3parser.d.ts` and copied into
//! the npm package on build (`just build-wasm`).
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

use serde::Serialize;
use war3parser::prelude::War3MapMetadata;
use wasm_bindgen::prelude::*;

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, String> {
    // Keep Option::None as `null` (familiar JSON shape for JS consumers).
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    value.serialize(&serializer).map_err(|err| err.to_string())
}

/// Parse a full War3 map (`.w3x` / `.w3m`) buffer.
///
/// Returns `undefined` if the buffer is not a readable MPQ/map.
/// On success, returns a plain JS object matching `War3MapMetadata` /
/// `MapSnapshot` in the package `.d.ts`.
#[wasm_bindgen]
pub fn parse_map(buffer: js_sys::Uint8Array) -> JsValue {
    let started = js_sys::Date::now();
    let Some(mut snapshot) = War3MapMetadata::parse_snapshot(&buffer.to_vec()) else {
        return JsValue::UNDEFINED;
    };
    snapshot.parse_ms = Some(js_sys::Date::now() - started);
    to_js(&snapshot).unwrap_or(JsValue::UNDEFINED)
}

/// Backward-compatible alias of [`parse_map`].
#[wasm_bindgen]
pub fn get_map_info(buffer: js_sys::Uint8Array) -> JsValue {
    parse_map(buffer)
}

/// Crate version string for UI badges.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

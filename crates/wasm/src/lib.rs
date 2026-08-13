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
use war3parser::prelude::{War3MapMetadata, War3MapW3x};
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
    let Ok(mut snapshot) = War3MapMetadata::parse_snapshot(&buffer.to_vec()) else {
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

/// Parse a map and keep the reason when it fails.
///
/// [`parse_map`] collapses every failure into `undefined`, which leaves a UI
/// with nothing to show beyond "parse failed". This returns
/// `{ ok: true, map }` or `{ ok: false, error }` instead.
#[wasm_bindgen]
pub fn parse_map_result(buffer: js_sys::Uint8Array) -> JsValue {
    let started = js_sys::Date::now();
    match War3MapMetadata::parse_snapshot(&buffer.to_vec()) {
        Ok(mut snapshot) => {
            snapshot.parse_ms = Some(js_sys::Date::now() - started);
            to_js(&ParseResult {
                ok: true,
                map: Some(snapshot),
                error: None,
            })
            .unwrap_or(JsValue::UNDEFINED)
        }
        Err(error) => to_js(&ParseResult {
            ok: false,
            map: None,
            error: Some(error.to_string()),
        })
        .unwrap_or(JsValue::UNDEFINED),
    }
}

#[derive(Serialize)]
struct ParseResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    map: Option<war3parser::model::MapSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// List the archive's `(listfile)` entries.
///
/// Returns `undefined` when the archive cannot be opened, and `null` when it
/// opens but carries no listfile — a common result for protected maps, where
/// files remain reachable by name even though they cannot be enumerated.
#[wasm_bindgen]
pub fn list_files(buffer: js_sys::Uint8Array) -> JsValue {
    let Ok(archive) = War3MapW3x::from_vec(buffer.to_vec()) else {
        return JsValue::UNDEFINED;
    };
    to_js(&archive.files).unwrap_or(JsValue::UNDEFINED)
}

/// Extract one file from the archive by its in-archive name, for example
/// `war3map.j` or `scripts\\war3map.j`.
///
/// Returns `undefined` when the archive cannot be opened or the file is absent.
#[wasm_bindgen]
pub fn extract_file(buffer: js_sys::Uint8Array, name: &str) -> JsValue {
    let Ok(mut archive) = War3MapW3x::from_vec(buffer.to_vec()) else {
        return JsValue::UNDEFINED;
    };
    match archive.read_file(name) {
        Ok(data) => js_sys::Uint8Array::from(data.as_slice()).into(),
        Err(_) => JsValue::UNDEFINED,
    }
}

/// Scan the map script for known third-party modifications.
///
/// Returns `undefined` when nothing matched. That is not a clean bill of
/// health: a protected map whose script cannot be read looks the same.
#[wasm_bindgen]
pub fn detect_modification(buffer: js_sys::Uint8Array) -> JsValue {
    let Ok(mut archive) = War3MapW3x::from_vec(buffer.to_vec()) else {
        return JsValue::UNDEFINED;
    };
    match war3parser::modscan::detect(&mut archive) {
        Some(found) => to_js(&found).unwrap_or(JsValue::UNDEFINED),
        None => JsValue::UNDEFINED,
    }
}

/// Crate version string for UI badges.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

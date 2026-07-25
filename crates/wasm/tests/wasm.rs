#![allow(dead_code)]

use war3parser::prelude::{War3MapMetadata, War3MapW3x};
use wasm_bindgen_test::*;

fn load_map() -> &'static [u8] {
    include_bytes!("../../../test_data/Legion_TD_11.1c_TeamOZE.w3x")
}

fn load_dota() -> &'static [u8] {
    include_bytes!("../../../test_data/DotA v6.83dAI PMV 1.42 EN.w3x")
}

#[wasm_bindgen_test]
fn test_w3x_parse() {
    let map = load_map();
    let w3x = War3MapW3x::from_buffer(map).expect("failed to parse w3x");
    assert!(w3x.header.has_hm3w);
}

#[wasm_bindgen_test]
fn test_wasm_mapinfo() {
    let map = load_map();
    let snapshot = War3MapMetadata::parse_snapshot(map).expect("failed to parse map info");
    assert!(snapshot.map_info.is_some());
    assert!(snapshot.header.has_hm3w);
    assert!(snapshot.strings.is_some());
}

#[wasm_bindgen_test]
fn test_dota_mapinfo() {
    let map = load_dota();
    let snapshot = War3MapMetadata::parse_snapshot(map).expect("dota metadata");
    let info = snapshot.map_info.expect("dota w3i");
    assert_eq!(info.version, 25);
    assert!(info.skipped_optional_sections);
}

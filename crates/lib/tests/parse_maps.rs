use war3parser::prelude::*;

const MAPS: &[(&str, u32, bool)] = &[
    // (path relative to crate, expected w3i version, expect_hm3w)
    ("../../test_data/DotA v6.83dAI PMV 1.42 EN.w3x", 25, true),
    ("../../test_data/Legion_TD_11.1c_TeamOZE.w3x", 31, true),
    ("../../test_data/TowerSurvivorsv1.71.w3x", 31, false),
];

#[test]
fn all_fixtures_parse_w3i() {
    for (path, expected_version, expect_hm3w) in MAPS {
        let buffer = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let mut w3x =
            War3MapW3x::from_buffer(&buffer).unwrap_or_else(|e| panic!("w3x {path}: {e}"));
        assert_eq!(
            w3x.has_hm3w, *expect_hm3w,
            "{path} hm3w expected {expect_hm3w}"
        );

        let info = w3x
            .read_map_info()
            .unwrap_or_else(|e| panic!("w3i {path}: {e:?}"));
        assert_eq!(info.version, *expected_version, "{path} w3i version");
        assert!(!info.players.is_empty(), "{path} has players");
        assert!(!info.forces.is_empty(), "{path} has forces");
        println!(
            "OK {path}: v{} name={:?} players={} forces={} skipped={}",
            info.version,
            info.name,
            info.players.len(),
            info.forces.len(),
            info.skipped_optional_sections
        );
    }
}

#[test]
fn metadata_roundtrip_string_table() {
    for (path, _, _) in MAPS {
        let buffer = std::fs::read(path).unwrap();
        let mut meta = War3MapMetadata::from(&buffer).expect("metadata");
        assert!(meta.map_info.is_some(), "{path} map_info");
        // WTS may or may not resolve TRIGSTR; just ensure call is safe
        let _ = meta.update_string_table();
        assert!(meta.wts.is_some(), "{path} should have wts");
    }
}

#[test]
fn dota_skips_optional_sections() {
    let path = "../../test_data/DotA v6.83dAI PMV 1.42 EN.w3x";
    let buffer = std::fs::read(path).unwrap();
    let mut w3x = War3MapW3x::from_buffer(&buffer).unwrap();
    let info = w3x.read_map_info().unwrap();
    assert!(info.skipped_optional_sections);
    assert!(info.upgrade_availability_changes.is_empty());
    assert!(info.random_item_tables.is_empty());
}

#[test]
fn legion_wts_has_many_strings() {
    let path = "../../test_data/Legion_TD_11.1c_TeamOZE.w3x";
    let buffer = std::fs::read(path).unwrap();
    let mut w3x = War3MapW3x::from_buffer(&buffer).unwrap();
    let wts = w3x.read_string_table().unwrap();
    // Old regex only matched ~88; brace parser should get thousands
    assert!(
        wts.string_map.len() > 500,
        "expected rich WTS, got {}",
        wts.string_map.len()
    );
}

#[test]
fn tower_has_imports() {
    let path = "../../test_data/TowerSurvivorsv1.71.w3x";
    let buffer = std::fs::read(path).unwrap();
    let mut w3x = War3MapW3x::from_buffer(&buffer).unwrap();
    let imp = w3x.read_imports().unwrap();
    assert!(!imp.entries.is_empty());
}

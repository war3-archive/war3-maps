use war3parser::formats::w3i::FormatVersion;
use war3parser::prelude::*;

const MAPS: &[(&str, FormatVersion, bool)] = &[
    // (path relative to crate, expected w3i version, expect_hm3w)
    (
        "../../test_data/DotA v6.83dAI PMV 1.42 EN.w3x",
        FormatVersion::V25,
        true,
    ),
    (
        "../../test_data/Legion_TD_11.1c_TeamOZE.w3x",
        FormatVersion::V31,
        true,
    ),
    (
        "../../test_data/TowerSurvivorsv1.71.w3x",
        FormatVersion::V31,
        false,
    ),
];

#[test]
fn all_fixtures_parse_w3i() {
    for (path, expected_version, expect_hm3w) in MAPS {
        let buffer = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let mut w3x =
            War3MapW3x::from_buffer(&buffer).unwrap_or_else(|e| panic!("w3x {path}: {e}"));
        assert_eq!(
            w3x.header.has_hm3w, *expect_hm3w,
            "{path} hm3w expected {expect_hm3w}"
        );

        let info = w3x
            .read_map_info()
            .unwrap_or_else(|e| panic!("w3i {path}: {e:?}"));
        assert_eq!(info.version, *expected_version, "{path} w3i version");
        assert!(!info.players.is_empty(), "{path} has players");
        assert!(!info.forces.is_empty(), "{path} has forces");
        println!(
            "OK {path}: {} name={:?} players={} forces={} skipped={}",
            info.version,
            info.name,
            info.players.len(),
            info.forces.len(),
            info.skipped_optional_sections
        );
    }
}

#[test]
fn metadata_resolves_trigger_strings() {
    for (path, _, _) in MAPS {
        let buffer = std::fs::read(path).unwrap();
        let mut meta = War3MapMetadata::parse(&buffer).expect("metadata");
        assert!(meta.map_info.is_some(), "{path} map_info");
        assert!(meta.wts.is_some(), "{path} should have wts");
        meta.resolve_trigger_strings();
        let info = meta.map_info.as_ref().unwrap();
        assert!(
            !info.name.starts_with("TRIGSTR_"),
            "{path} name still unresolved: {:?}",
            info.name
        );
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

#[test]
fn tower_minimap_icons() {
    let path = "../../test_data/TowerSurvivorsv1.71.w3x";
    let buffer = std::fs::read(path).unwrap();
    let mut w3x = War3MapW3x::from_buffer(&buffer).unwrap();
    let mmp = w3x.read_minimap_icons().expect("mmp");
    assert_eq!(mmp.icons.len(), 8);
    assert!(mmp.icons.iter().all(|i| i.icon_type == 2));
    // first tower player should be roughly red-ish after BGRA→RGBA
    let c = mmp.icons[0].color;
    assert!(c[0] > 200, "expected red channel high, got {c:?}");
}

#[test]
fn dota_minimap_has_buildings_and_starts() {
    let path = "../../test_data/DotA v6.83dAI PMV 1.42 EN.w3x";
    let buffer = std::fs::read(path).unwrap();
    let meta = War3MapMetadata::parse(&buffer).unwrap();
    let mmp = meta.minimap_icons.expect("mmp");
    assert!(mmp.icons.iter().any(|i| i.icon_type == 1));
    assert!(mmp.icons.iter().any(|i| i.icon_type == 2));
}

#[test]
fn snapshot_shape_is_stable() {
    let path = "../../test_data/TowerSurvivorsv1.71.w3x";
    let buffer = std::fs::read(path).unwrap();
    let snapshot = War3MapMetadata::parse_snapshot(&buffer).unwrap();
    assert!(snapshot.map_info.is_some());
    assert!(!snapshot.images.is_empty());
    assert!(snapshot.imports.as_ref().is_some_and(|i| !i.is_empty()));
    assert!(snapshot.strings.as_ref().is_some_and(|s| !s.is_empty()));
    assert!(snapshot.files.as_ref().is_some_and(|f| !f.is_empty()));

    // w3i version must serialize as a plain number (WASM d.ts contract)
    let json = serde_json::to_value(&snapshot).unwrap();
    assert!(json["map_info"]["version"].is_number());
}

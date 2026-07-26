//! Synthetic-buffer coverage for the full w3i version ladder (v8 → v33).
//!
//! Real fixture maps only cover v25 and v31; these tests build byte-exact
//! buffers for every other version per the War3Net layout and assert that the
//! gated fields land where they should.

use war3parser::formats::w3i::{FormatVersion, War3MapW3i};

/// Minimal little-endian writer mirroring the documented w3i layout.
#[derive(Default)]
struct W3iWriter {
    buf: Vec<u8>,
}

impl W3iWriter {
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.buf.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.buf.extend(v.to_le_bytes());
    }
    fn cstr(&mut self, s: &str) {
        self.buf.extend(s.as_bytes());
        self.buf.push(0);
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend(bytes);
    }
}

/// Build a syntactically complete w3i buffer for `version` with one player
/// and one force, following the War3Net field order.
fn build_w3i(version: u32) -> Vec<u8> {
    let v = version;
    let mut w = W3iWriter::default();

    w.u32(v);
    if v >= 18 {
        w.u32(5); // saves
        w.u32(6059); // editor version
    }
    if v >= 27 {
        w.u32(1);
        w.u32(32);
        w.u32(10);
        w.u32(12345); // build version 1.32.10.12345
    }

    w.cstr("Test Map");
    w.cstr("An Author");
    w.cstr("A description");
    w.cstr("1v1");

    if v == 8 {
        w.f32(1.0);
        w.i32(2);
        w.f32(3.0);
        w.f32(4.0);
        w.f32(5.0);
        w.i32(6);
    }

    for i in 0..8 {
        w.f32(i as f32 * 100.0); // camera bounds
    }
    if v >= 15 {
        for i in 0..4 {
            w.i32(i); // camera bounds complements
        }
    }
    w.i32(64); // playable width
    w.i32(64); // playable height
    if v == 8 {
        w.i32(7);
    }
    w.u32(0x8000); // flags
    w.u8(b'L'); // tileset

    if v >= 23 {
        w.i32(-1); // loading screen background
        w.cstr("LoadingScreen.mdx");
    } else if v >= 18 {
        w.i32(3); // campaign background
    } else if v >= 15 {
        w.cstr("LoadingScreen.mdx");
    }

    if v >= 10 {
        w.cstr("loading text");
        w.cstr("loading title");
        if v >= 15 {
            w.cstr("loading subtitle");
        }

        if v >= 23 {
            w.i32(2); // game data set (melee)
            w.cstr("Prologue.mdx");
        } else if v >= 18 {
            w.i32(1); // loading screen index
        } else if v >= 15 {
            w.cstr("Prologue.mdx");
        }

        if v >= 11 {
            w.cstr("prologue text");
            w.cstr("prologue title");
            if v >= 15 {
                w.cstr("prologue subtitle");
            }
        }

        if v >= 23 {
            w.i32(1); // fog style
            w.f32(3000.0); // fog start z
            w.f32(5000.0); // fog end z
            w.f32(0.5); // fog density
            w.raw(&[10, 20, 30, 255]); // fog color (BGRA)
            if v >= 25 {
                w.i32(0x4C726169); // global weather 'RAil'-ish rawcode
            }
            w.cstr("LordaeronSummerDay");
            w.u8(b'L'); // light environment tileset
            w.raw(&[0, 64, 128, 255]); // water color (BGRA)
        }

        if v >= 28 {
            w.u32(1); // script mode: Lua
        }
        if v >= 31 {
            w.u32(3); // graphics mode: SD+HD
            w.u32(1); // game data version: TFT
        }
        if v >= 32 {
            w.i32(1650); // default camera zoom
            w.i32(3000); // max camera zoom
        }
        if v >= 33 {
            w.i32(1200); // min camera zoom
        }
    }

    // one player
    w.u32(1);
    w.i32(0); // id
    w.i32(1); // user
    w.i32(1); // human
    w.i32(1); // fixed start
    w.cstr("Player 1");
    w.f32(-1024.0);
    w.f32(1024.0);
    w.u32(0); // ally low
    w.u32(0); // ally high
    if v >= 31 {
        w.u32(0); // enemy low
        w.u32(0); // enemy high
    }

    // one force
    w.u32(1);
    w.u32(0b11); // flags
    w.u32(0xFFFF_FFFF); // player mask
    w.cstr("Force 1");

    // one upgrade change, one tech change
    w.u32(1);
    w.u32(1); // players
    w.raw(b"Rhme");
    w.i32(0); // level
    w.i32(2); // researched
    w.u32(1);
    w.u32(1); // players
    w.raw(b"hgtw");

    if v >= 15 {
        w.u32(0); // random unit tables
    }
    if v >= 24 {
        w.u32(0); // random item tables
    }
    if (26..28).contains(&v) {
        w.i32(0); // trailing zero int
    }

    w.buf
}

const ALL_VERSIONS: &[u32] = &[8, 10, 11, 15, 18, 23, 24, 25, 26, 27, 28, 31, 32, 33];

#[test]
fn every_known_version_parses_completely() {
    for &v in ALL_VERSIONS {
        let buf = build_w3i(v);
        let info = War3MapW3i::parse(&buf).unwrap_or_else(|e| panic!("v{v}: {e}"));

        assert_eq!(info.version, FormatVersion(v));
        assert_eq!(info.name, "Test Map", "v{v} name");
        assert_eq!(info.recommended_players, "1v1", "v{v} recommended");
        assert_eq!(info.playable_size, [64, 64], "v{v} playable size");
        assert_eq!(info.flags, 0x8000, "v{v} flags");
        assert_eq!(info.tileset, b'L', "v{v} tileset");

        assert_eq!(info.players.len(), 1, "v{v} players");
        assert_eq!(info.players[0].name, "Player 1", "v{v} player name");
        assert_eq!(info.forces.len(), 1, "v{v} forces");
        assert_eq!(info.forces[0].name, "Force 1", "v{v} force name");
        assert_eq!(
            info.upgrade_availability_changes.len(),
            1,
            "v{v} upgrade changes"
        );
        assert_eq!(info.upgrade_availability_changes[0].id, *b"Rhme");
        assert_eq!(info.tech_availability_changes.len(), 1, "v{v} tech changes");
        assert_eq!(info.tech_availability_changes[0].id, *b"hgtw");
        assert!(!info.skipped_optional_sections, "v{v} not skipped");
    }
}

#[test]
fn version_gates_toggle_fields() {
    for &v in ALL_VERSIONS {
        let info = War3MapW3i::parse(&build_w3i(v)).unwrap();
        let fv = FormatVersion(v);

        assert_eq!(info.saves.is_some(), v >= 18, "v{v} saves");
        assert_eq!(info.build_version.is_some(), v >= 27, "v{v} build version");
        assert_eq!(info.legacy_v8.is_some(), v == 8, "v{v} legacy fields");
        assert_eq!(
            info.camera_bounds_complements.is_some(),
            v >= 15,
            "v{v} complements"
        );
        assert_eq!(
            info.loading_screen_background.is_some(),
            v >= 23,
            "v{v} loading bg"
        );
        assert_eq!(
            info.campaign_background.is_some(),
            (18..23).contains(&v),
            "v{v} campaign bg"
        );
        assert_eq!(
            info.loading_screen_model.is_some(),
            (15..18).contains(&v) || v >= 23,
            "v{v} loading model"
        );
        assert_eq!(
            info.loading_screen_text.is_some(),
            v >= 10,
            "v{v} loading text"
        );
        assert_eq!(
            info.loading_screen_subtitle.is_some(),
            v >= 15,
            "v{v} loading subtitle"
        );
        assert_eq!(info.game_data_set.is_some(), v >= 23, "v{v} game data set");
        assert_eq!(
            info.loading_screen_index.is_some(),
            (18..23).contains(&v),
            "v{v} loading index"
        );
        assert_eq!(
            info.prologue_screen_text.is_some(),
            v >= 11,
            "v{v} prologue text"
        );
        assert_eq!(info.fog_style.is_some(), v >= 23, "v{v} fog");
        assert_eq!(info.global_weather.is_some(), v >= 25, "v{v} weather");
        assert_eq!(info.script_mode.is_some(), v >= 28, "v{v} script mode");
        assert_eq!(info.graphics_mode.is_some(), v >= 31, "v{v} graphics mode");
        assert_eq!(
            info.default_camera_zoom.is_some(),
            v >= 32,
            "v{v} default zoom"
        );
        assert_eq!(info.min_camera_zoom.is_some(), v >= 33, "v{v} min zoom");
        assert_eq!(
            info.players[0].enemy_low_priorities.is_some(),
            v >= 31,
            "v{v} enemy priorities"
        );

        // era helpers stay consistent with the ladder
        assert_eq!(fv.is_tft(), v >= 23, "v{v} is_tft");
        assert_eq!(fv.is_reforged(), v >= 31, "v{v} is_reforged");
    }
}

#[test]
fn gated_values_roundtrip_exactly() {
    let info = War3MapW3i::parse(&build_w3i(33)).unwrap();
    assert_eq!(info.saves, Some(5));
    assert_eq!(info.editor_version, Some(6059));
    assert_eq!(info.build_version, Some([1, 32, 10, 12345]));
    assert_eq!(info.get_build_version(), 132);
    assert_eq!(info.loading_screen_background, Some(-1));
    assert_eq!(
        info.loading_screen_model.as_deref(),
        Some("LoadingScreen.mdx")
    );
    assert_eq!(info.game_data_set, Some(2));
    assert_eq!(info.fog_style, Some(1));
    assert_eq!(info.fog_height, Some([3000.0, 5000.0]));
    assert_eq!(info.fog_density, Some(0.5));
    assert_eq!(info.fog_color, Some([10, 20, 30, 255]));
    assert_eq!(
        info.sound_environment.as_deref(),
        Some("LordaeronSummerDay")
    );
    assert_eq!(info.light_environment_tileset, Some(b'L'));
    assert_eq!(info.script_mode, Some(1));
    assert_eq!(info.graphics_mode, Some(3));
    assert_eq!(info.game_data_version, Some(1));
    assert_eq!(info.default_camera_zoom, Some(1650));
    assert_eq!(info.max_camera_zoom, Some(3000));
    assert_eq!(info.min_camera_zoom, Some(1200));
}

#[test]
fn v8_legacy_fields_roundtrip() {
    let info = War3MapW3i::parse(&build_w3i(8)).unwrap();
    let legacy = info.legacy_v8.expect("v8 legacy fields");
    assert_eq!(legacy.unk2, 2);
    assert_eq!(legacy.unk6, 6);
    assert_eq!(legacy.unk7, 7);
    // v8 predates every optional block
    assert!(info.saves.is_none());
    assert!(info.loading_screen_subtitle.is_none());
    assert!(info.fog_style.is_none());
}

#[test]
fn skip_marker_after_forces_is_honored() {
    // Rebuild a v25 buffer but replace everything after the forces with 0xFF.
    let mut w = build_w3i(25);
    // upgrade(1×16B hdr+count) + tech + unit tables + item tables = trailing:
    // count(4)+16 + count(4)+8 + count(4) + count(4) = 40 bytes
    let cut = w.len() - 40;
    w.truncate(cut);
    w.push(0xFF);

    let info = War3MapW3i::parse(&w).unwrap();
    assert!(info.skipped_optional_sections);
    assert!(info.upgrade_availability_changes.is_empty());
    assert!(info.tech_availability_changes.is_empty());
    assert_eq!(info.players.len(), 1);
}

#[test]
fn truncation_after_forces_yields_empty_sections() {
    let mut w = build_w3i(31);
    let cut = w.len() - 40;
    w.truncate(cut);

    let info = War3MapW3i::parse(&w).unwrap();
    assert!(!info.skipped_optional_sections);
    assert!(info.upgrade_availability_changes.is_empty());
    assert_eq!(info.forces.len(), 1);
}

#[test]
fn unknown_future_version_parses_with_nearest_layout() {
    // v29/v30 don't exist in the wild but should follow the v28 layout.
    let mut buf = build_w3i(28);
    buf[0..4].copy_from_slice(&29u32.to_le_bytes());
    let info = War3MapW3i::parse(&buf).unwrap();
    assert_eq!(info.version, FormatVersion(29));
    assert!(!info.version.is_known());
    assert_eq!(info.script_mode, Some(1));
    assert!(info.graphics_mode.is_none());
}

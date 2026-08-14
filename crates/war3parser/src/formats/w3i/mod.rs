//! `war3map.w3i` — map information file.
//!
//! Field order and version gates are ported from
//! [War3Net](https://github.com/Drake53/War3Net)
//! (`War3Net.Build.Core/Serialization/Binary/Info/MapInfo.cs`), the most
//! complete open reference. The full ladder v8 → v33 is supported:
//! see [`FormatVersion`] for what each version corresponds to.

mod availability;
mod force;
mod player;
mod random_tables;
mod version;

pub use availability::{TechAvailabilityChange, UpgradeAvailabilityChange};
pub use force::Force;
pub use player::Player;
pub use random_tables::{RandomItem, RandomItemSet, RandomItemTable, RandomUnit, RandomUnitTable};
pub use version::FormatVersion;

use crate::error::Result;
use crate::reader::{parse_counted, ByteReader};

/// Unknown legacy fields only present in v8 (Reign of Chaos beta) files.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct LegacyV8Fields {
    pub unk1: f32,
    pub unk2: i32,
    pub unk3: f32,
    pub unk4: f32,
    pub unk5: f32,
    pub unk6: i32,
    /// Read separately, after the playable map size.
    pub unk7: i32,
}

/// Parsed `war3map.w3i` map information.
///
/// Fields are declared in file order. `Option` fields only exist in the
/// version range noted on each field; everything else is present in all
/// supported versions (v8+).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct War3MapW3i {
    pub version: FormatVersion,
    /// Save counter (v18+).
    pub saves: Option<u32>,
    /// World Editor version that saved the map (v18+).
    pub editor_version: Option<u32>,
    /// Game version `[major, minor, patch, build]` (v27+).
    pub build_version: Option<[u32; 4]>,

    pub name: String,
    pub author: String,
    pub description: String,
    pub recommended_players: String,

    /// Unknown legacy fields (v8 only).
    pub legacy_v8: Option<LegacyV8Fields>,

    pub camera_bounds: [f32; 8],
    /// Camera bounds complements (v15+).
    pub camera_bounds_complements: Option<[i32; 4]>,
    pub playable_size: [i32; 2],
    pub flags: u32,
    pub tileset: u8,

    /// Loading screen background index; -1 = imported model (v23+).
    pub loading_screen_background: Option<i32>,
    /// Campaign background index (v18–v22 only; became
    /// `loading_screen_background` in v23).
    pub campaign_background: Option<i32>,
    /// Path of an imported loading screen model (v15–v17, v23+).
    pub loading_screen_model: Option<String>,

    /// Loading screen text (v10+).
    pub loading_screen_text: Option<String>,
    /// Loading screen title (v10+).
    pub loading_screen_title: Option<String>,
    /// Loading screen subtitle (v15+).
    pub loading_screen_subtitle: Option<String>,

    /// Game data set: 0 = default, 1 = custom, 2 = melee (v23+).
    pub game_data_set: Option<i32>,
    /// Loading screen index (v18–v22 only; superseded by
    /// `loading_screen_background` + `game_data_set` in v23).
    pub loading_screen_index: Option<i32>,

    /// Path of an imported prologue screen model (v15–v17, v23+).
    pub prologue_screen_model: Option<String>,
    /// Prologue screen text (v11+).
    pub prologue_screen_text: Option<String>,
    /// Prologue screen title (v11+).
    pub prologue_screen_title: Option<String>,
    /// Prologue screen subtitle (v15+).
    pub prologue_screen_subtitle: Option<String>,

    /// Terrain fog style: 0 = none, 1 = linear, … (v23+).
    pub fog_style: Option<i32>,
    /// Fog start / end Z (v23+).
    pub fog_height: Option<[f32; 2]>,
    /// Fog density (v23+).
    pub fog_density: Option<f32>,
    /// Fog color, file order BGRA (v23+).
    pub fog_color: Option<[u8; 4]>,
    /// Global weather rawcode, 0 = none (v25+).
    pub global_weather: Option<i32>,
    /// Custom sound environment (v23+).
    pub sound_environment: Option<String>,
    /// Tileset id of the light environment (v23+).
    pub light_environment_tileset: Option<u8>,
    /// Water tinting color, file order BGRA (v23+).
    pub water_vertex_color: Option<[u8; 4]>,

    /// Script language: 0 = JASS, 1 = Lua (v28+).
    pub script_mode: Option<u32>,
    /// Supported graphics modes: 1 = SD, 2 = HD, 3 = both (v31+).
    pub graphics_mode: Option<u32>,
    /// Game data version: 0 = ROC, 1 = TFT (v31+).
    pub game_data_version: Option<u32>,

    /// Forced default camera zoom (v32+).
    pub default_camera_zoom: Option<i32>,
    /// Forced maximum camera zoom (v32+).
    pub max_camera_zoom: Option<i32>,
    /// Forced minimum camera zoom (v33+).
    pub min_camera_zoom: Option<i32>,

    /// True when the trailing upgrade/tech/random sections were omitted via
    /// the `0xFF` marker (common on older / protected maps such as DotA).
    pub skipped_optional_sections: bool,

    pub players: Vec<Player>,
    pub forces: Vec<Force>,
    pub upgrade_availability_changes: Vec<UpgradeAvailabilityChange>,
    pub tech_availability_changes: Vec<TechAvailabilityChange>,
    /// Random unit tables (v15+).
    pub random_unit_tables: Vec<RandomUnitTable>,
    /// Random item tables (v24+).
    pub random_item_tables: Vec<RandomItemTable>,
}

/// The string-delimited tail of a `w3i`: players, forces, and the optional
/// availability/random-table sections.
///
/// Split out from [`War3MapW3i::parse_reader`] so it can be attempted twice
/// against different empty-string encodings without duplicating the field list.
struct Sections {
    players: Vec<Player>,
    forces: Vec<Force>,
    skipped_optional_sections: bool,
    upgrade_availability_changes: Vec<UpgradeAvailabilityChange>,
    tech_availability_changes: Vec<TechAvailabilityChange>,
    random_unit_tables: Vec<RandomUnitTable>,
    random_item_tables: Vec<RandomItemTable>,
}

impl Sections {
    fn parse(r: &mut ByteReader<'_>, v: FormatVersion) -> Result<Self> {
        use FormatVersion as V;

        let players = parse_counted(r, |r| Player::parse(r, v))?;
        let forces = parse_counted(r, Force::parse)?;

        // A single 0xFF byte after the forces marks the remaining sections as
        // omitted (War3Net `_skipData`; typical of protected maps).
        let mut skipped_optional_sections = false;
        let mut upgrade_availability_changes = Vec::new();
        let mut tech_availability_changes = Vec::new();
        let mut random_unit_tables = Vec::new();
        let mut random_item_tables = Vec::new();

        if r.peek_u8() == Some(0xFF) {
            r.skip(1)?;
            skipped_optional_sections = true;
        } else {
            upgrade_availability_changes = parse_counted(r, UpgradeAvailabilityChange::parse)?;
            // Some maps end right after the upgrade section (War3Net early-returns here).
            if !r.is_at_end() {
                tech_availability_changes = parse_counted(r, TechAvailabilityChange::parse)?;
                if v >= V::V15 {
                    random_unit_tables = parse_counted(r, RandomUnitTable::parse)?;
                }
                if v >= V::V24 {
                    random_item_tables = parse_counted(r, RandomItemTable::parse)?;
                }
                // v26/v27 append a trailing zero int; read and ignore it.
                if v >= V::V26 && v < V::V28 && r.remaining() >= 4 {
                    let _ = r.i32()?;
                }
            }
        }

        Ok(Self {
            players,
            forces,
            skipped_optional_sections,
            upgrade_availability_changes,
            tech_availability_changes,
            random_unit_tables,
            random_item_tables,
        })
    }
}

impl War3MapW3i {
    /// Parse a complete `war3map.w3i` buffer.
    pub fn parse(data: &[u8]) -> Result<Self> {
        Self::parse_reader(&mut ByteReader::new(data))
    }

    fn parse_reader(r: &mut ByteReader<'_>) -> Result<Self> {
        use FormatVersion as V;

        let version = V(r.u32()?);
        let v = version;

        let (saves, editor_version) = if v >= V::V18 {
            (Some(r.u32()?), Some(r.u32()?))
        } else {
            (None, None)
        };
        let build_version = (v >= V::V27).then(|| r.u32s()).transpose()?;

        let name = r.cstr_lossy()?;
        let author = r.cstr_lossy()?;
        let description = r.cstr_lossy()?;
        let recommended_players = r.cstr_lossy()?;

        let mut legacy_v8 = (v == V::V8)
            .then(|| -> Result<LegacyV8Fields> {
                Ok(LegacyV8Fields {
                    unk1: r.f32()?,
                    unk2: r.i32()?,
                    unk3: r.f32()?,
                    unk4: r.f32()?,
                    unk5: r.f32()?,
                    unk6: r.i32()?,
                    unk7: 0,
                })
            })
            .transpose()?;

        let camera_bounds = r.f32s()?;
        let camera_bounds_complements = (v >= V::V15).then(|| r.i32s()).transpose()?;
        let playable_size = r.i32s()?;

        if let Some(legacy) = legacy_v8.as_mut() {
            legacy.unk7 = r.i32()?;
        }

        let flags = r.u32()?;
        let tileset = r.u8()?;

        // v23+: loading screen background index + model path.
        // v18–v22: campaign background index only.
        // v15–v17: model path only.
        let (loading_screen_background, campaign_background, loading_screen_model) = if v >= V::V23
        {
            (Some(r.i32()?), None, Some(r.cstr_lossy()?))
        } else if v >= V::V18 {
            (None, Some(r.i32()?), None)
        } else if v >= V::V15 {
            (None, None, Some(r.cstr_lossy()?))
        } else {
            (None, None, None)
        };

        let mut loading_screen_text = None;
        let mut loading_screen_title = None;
        let mut loading_screen_subtitle = None;
        let mut game_data_set = None;
        let mut loading_screen_index = None;
        let mut prologue_screen_model = None;
        let mut prologue_screen_text = None;
        let mut prologue_screen_title = None;
        let mut prologue_screen_subtitle = None;
        let mut fog_style = None;
        let mut fog_height = None;
        let mut fog_density = None;
        let mut fog_color = None;
        let mut global_weather = None;
        let mut sound_environment = None;
        let mut light_environment_tileset = None;
        let mut water_vertex_color = None;
        let mut script_mode = None;
        let mut graphics_mode = None;
        let mut game_data_version = None;
        let mut default_camera_zoom = None;
        let mut max_camera_zoom = None;
        let mut min_camera_zoom = None;

        if v >= V::V10 {
            loading_screen_text = Some(r.cstr_lossy()?);
            loading_screen_title = Some(r.cstr_lossy()?);
            if v >= V::V15 {
                loading_screen_subtitle = Some(r.cstr_lossy()?);
            }

            // v23+: game data set + prologue path.
            // v18–v22: loading screen index only.
            // v15–v17: prologue path only.
            if v >= V::V23 {
                game_data_set = Some(r.i32()?);
                prologue_screen_model = Some(r.cstr_lossy()?);
            } else if v >= V::V18 {
                loading_screen_index = Some(r.i32()?);
            } else if v >= V::V15 {
                prologue_screen_model = Some(r.cstr_lossy()?);
            }

            if v >= V::V11 {
                prologue_screen_text = Some(r.cstr_lossy()?);
                prologue_screen_title = Some(r.cstr_lossy()?);
                if v >= V::V15 {
                    prologue_screen_subtitle = Some(r.cstr_lossy()?);
                }
            }

            if v >= V::V23 {
                fog_style = Some(r.i32()?);
                fog_height = Some(r.f32s()?);
                fog_density = Some(r.f32()?);
                fog_color = Some(r.bytes()?);
                if v >= V::V25 {
                    global_weather = Some(r.i32()?);
                }
                sound_environment = Some(r.cstr_lossy()?);
                light_environment_tileset = Some(r.u8()?);
                water_vertex_color = Some(r.bytes()?);
            }

            if v >= V::V28 {
                script_mode = Some(r.u32()?);
            }

            if v >= V::V31 {
                graphics_mode = Some(r.u32()?);
                game_data_version = Some(r.u32()?);
            }

            if v >= V::V32 {
                default_camera_zoom = Some(r.i32()?);
                max_camera_zoom = Some(r.i32()?);
            }
            if v >= V::V33 {
                min_camera_zoom = Some(r.i32()?);
            }
        }

        // Everything from here on is length-delimited by embedded strings, so a
        // writer that mis-encodes one shifts every field after it. Parse on a
        // clone and retry leniently rather than let that surface as a truncated
        // file — see `Sections::parse` and `set_empty_strings_unterminated`.
        let sections = {
            let mut attempt = r.clone();
            match Sections::parse(&mut attempt, v) {
                Ok(sections) => {
                    *r = attempt;
                    Ok(sections)
                }
                Err(strict_error) => {
                    let mut attempt = r.clone();
                    attempt.set_empty_strings_unterminated(true);
                    match Sections::parse(&mut attempt, v) {
                        Ok(sections) => {
                            *r = attempt;
                            Ok(sections)
                        }
                        // Report what the conformant read saw; the retry is an
                        // accommodation, not the standard we parse against.
                        Err(_) => Err(strict_error),
                    }
                }
            }
        }?;
        let Sections {
            players,
            forces,
            skipped_optional_sections,
            upgrade_availability_changes,
            tech_availability_changes,
            random_unit_tables,
            random_item_tables,
        } = sections;

        Ok(Self {
            version,
            saves,
            editor_version,
            build_version,
            name,
            author,
            description,
            recommended_players,
            legacy_v8,
            camera_bounds,
            camera_bounds_complements,
            playable_size,
            flags,
            tileset,
            loading_screen_background,
            campaign_background,
            loading_screen_model,
            loading_screen_text,
            loading_screen_title,
            loading_screen_subtitle,
            game_data_set,
            loading_screen_index,
            prologue_screen_model,
            prologue_screen_text,
            prologue_screen_title,
            prologue_screen_subtitle,
            fog_style,
            fog_height,
            fog_density,
            fog_color,
            global_weather,
            sound_environment,
            light_environment_tileset,
            water_vertex_color,
            script_mode,
            graphics_mode,
            game_data_version,
            default_camera_zoom,
            max_camera_zoom,
            min_camera_zoom,
            skipped_optional_sections,
            players,
            forces,
            upgrade_availability_changes,
            tech_availability_changes,
            random_unit_tables,
            random_item_tables,
        })
    }

    /// Game build as `major * 100 + minor` (e.g. 1.32 → 132), 0 when absent.
    pub fn get_build_version(&self) -> u32 {
        match self.build_version {
            Some(version) => version[0] * 100 + version[1],
            None => 0,
        }
    }

    /// Visit every user-facing string in place (used for TRIGSTR resolution).
    pub fn visit_strings(&mut self, mut f: impl FnMut(&mut String)) {
        f(&mut self.name);
        f(&mut self.author);
        f(&mut self.description);
        f(&mut self.recommended_players);
        for s in [
            &mut self.loading_screen_text,
            &mut self.loading_screen_title,
            &mut self.loading_screen_subtitle,
            &mut self.prologue_screen_text,
            &mut self.prologue_screen_title,
            &mut self.prologue_screen_subtitle,
        ]
        .into_iter()
        .flatten()
        {
            f(s);
        }
        for player in &mut self.players {
            f(&mut player.name);
        }
        for force in &mut self.forces {
            f(&mut force.name);
        }
        for table in &mut self.random_unit_tables {
            f(&mut table.name);
        }
        for table in &mut self.random_item_tables {
            f(&mut table.name);
        }
    }
}

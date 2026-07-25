pub mod force;
pub mod player;
pub mod random_item_table;
pub mod random_unit_table;
pub mod tech_availability_change;
pub mod upgrade_availability_change;

use std::collections::HashMap;

use binary_reader::BinaryReader;

use crate::parser::binary_reader::{AutoReadable, BinaryReadable};

use super::error::ParserError;

use {
    force::Force, player::Player, random_item_table::RandomItemTable,
    random_unit_table::RandomUnitTable, tech_availability_change::TechAvailabilityChange,
    upgrade_availability_change::UpgradeAvailabilityChange,
};

/// Map info for `war3map.w3i` file
///
/// Special thanks to [mdx-m3-viewer](https://github.com/flowtsohg/mdx-m3-viewer) for the map info format.
/// Version notes:
/// - 18: ROC
/// - 25: TFT (loading models, fog, random item tables)
/// - 28: build version + script language
/// - 31: Reforged supported modes + game data version; player enemy priorities
/// - 32/33: camera zoom defaults (WC3 2.0)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct War3MapW3i {
    pub version: u32,
    pub saves: u32,
    pub editor_version: u32,
    /// Present when `version > 27` (1.31+)
    pub build_version: Option<[u32; 4]>,

    pub name: String,
    pub author: String,
    pub description: String,
    pub recommended_players: String,

    pub camera_bounds: [f32; 8],
    pub camera_bounds_complements: [i32; 4],
    pub playable_size: [i32; 2],

    pub flags: u32,
    pub tileset: u8,

    /// Loading screen background index (ROC) / custom loading index
    pub campaign_background: i32,

    /// Present when `version > 24` (TFT+)
    pub loading_screen_model: Option<String>,
    pub loading_screen_text: String,
    pub loading_screen_title: String,
    pub loading_screen_subtitle: String,
    /// Game data set on TFT+ (0 default, 1 custom, 2 melee)
    pub loading_screen: i32,

    pub prologue_screen_model: Option<String>,
    pub prologue_screen_text: String,
    pub prologue_screen_title: String,
    pub prologue_screen_subtitle: String,

    /// Fog style when `version > 24`
    pub use_terrain_fog: Option<i32>,
    pub fog_height: Option<[f32; 2]>,
    pub fog_density: Option<f32>,
    pub fog_color: Option<[u8; 4]>,
    pub global_weather: Option<i32>,
    pub sound_environment: Option<String>,
    pub light_environment_tileset: Option<u8>,
    pub water_vertex_color: Option<[u8; 4]>,

    /// 0 = JASS, 1 = Lua (`version > 27`)
    pub script_mode: Option<u32>,

    /// Supported graphics modes (`version > 30`): 1=SD, 2=HD, 3=both
    pub graphics_mode: Option<u32>,
    /// Game data version (`version > 30`): 0=ROC, 1=TFT
    pub game_data_version: Option<u32>,

    /// WC3 2.0 camera zoom (`version >= 32`)
    pub default_camera_zoom: Option<u32>,
    pub max_camera_zoom: Option<u32>,
    /// `version >= 33`
    pub min_camera_zoom: Option<u32>,

    /// True when trailing upgrade/tech/random sections were omitted (`0xFF` marker)
    pub skipped_optional_sections: bool,

    pub players: Vec<Player>,
    pub forces: Vec<Force>,
    pub upgrade_availability_changes: Vec<UpgradeAvailabilityChange>,
    pub tech_availability_changes: Vec<TechAvailabilityChange>,
    pub random_unit_tables: Vec<RandomUnitTable>,
    pub random_item_tables: Vec<RandomItemTable>,
}

fn remaining(stream: &BinaryReader) -> usize {
    stream.length.saturating_sub(stream.pos)
}

fn read_count_vec<T: BinaryReadable>(
    stream: &mut BinaryReader,
    version: u32,
) -> Result<Vec<T>, ParserError> {
    if remaining(stream) < 4 {
        return Ok(Vec::new());
    }
    let count: u32 = AutoReadable::read(stream)?;
    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        items.push(T::load(stream, version)?);
    }
    Ok(items)
}

impl BinaryReadable for War3MapW3i {
    fn load(stream: &mut BinaryReader, _version: u32) -> Result<Self, ParserError> {
        let version: u32 = AutoReadable::read(stream)?;
        let saves: u32 = AutoReadable::read(stream)?;
        let editor_version: u32 = AutoReadable::read(stream)?;
        let build_version = if version > 27 {
            Some(AutoReadable::read(stream)?)
        } else {
            None
        };

        let name: String = AutoReadable::read(stream)?;
        let author: String = AutoReadable::read(stream)?;
        let description: String = AutoReadable::read(stream)?;
        let recommended_players: String = AutoReadable::read(stream)?;
        let camera_bounds: [f32; 8] = AutoReadable::read(stream)?;
        let camera_bounds_complements: [i32; 4] = AutoReadable::read(stream)?;
        let playable_size: [i32; 2] = AutoReadable::read(stream)?;
        let flags: u32 = AutoReadable::read(stream)?;
        let tileset: u8 = AutoReadable::read(stream)?;
        let campaign_background: i32 = AutoReadable::read(stream)?;

        let loading_screen_model = if version > 24 {
            Some(AutoReadable::read(stream)?)
        } else {
            None
        };
        let loading_screen_text: String = AutoReadable::read(stream)?;
        let loading_screen_title: String = AutoReadable::read(stream)?;
        let loading_screen_subtitle: String = AutoReadable::read(stream)?;
        let loading_screen: i32 = AutoReadable::read(stream)?;

        let prologue_screen_model = if version > 24 {
            Some(AutoReadable::read(stream)?)
        } else {
            None
        };
        let prologue_screen_text: String = AutoReadable::read(stream)?;
        let prologue_screen_title: String = AutoReadable::read(stream)?;
        let prologue_screen_subtitle: String = AutoReadable::read(stream)?;

        let (
            use_terrain_fog,
            fog_height,
            fog_density,
            fog_color,
            global_weather,
            sound_environment,
            light_environment_tileset,
            water_vertex_color,
        ) = if version > 24 {
            (
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
            )
        } else {
            (None, None, None, None, None, None, None, None)
        };

        let script_mode = if version > 27 {
            Some(AutoReadable::read(stream)?)
        } else {
            None
        };

        let (graphics_mode, game_data_version) = if version > 30 {
            (
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
            )
        } else {
            (None, None)
        };

        // WC3 2.0 camera zoom — HiveWE / War3Net layout:
        // v32+: default + max; v33+: min
        let (default_camera_zoom, max_camera_zoom, min_camera_zoom) = if version >= 33 {
            (
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
            )
        } else if version >= 32 {
            (
                Some(AutoReadable::read(stream)?),
                Some(AutoReadable::read(stream)?),
                None,
            )
        } else {
            (None, None, None)
        };

        let players = read_count_vec::<Player>(stream, version)?;
        let forces = read_count_vec::<Force>(stream, version)?;

        // War3Net: after forces, a single 0xFF byte means upgrade/tech/random data is omitted
        // (common on older / protected maps such as classic DotA).
        let mut skipped_optional_sections = false;
        let mut upgrade_availability_changes = Vec::new();
        let mut tech_availability_changes = Vec::new();
        let mut random_unit_tables = Vec::new();
        let mut random_item_tables = Vec::new();

        if remaining(stream) > 0 && stream.data[stream.pos] == 0xFF {
            stream.adv(1);
            skipped_optional_sections = true;
        } else {
            upgrade_availability_changes =
                read_count_vec::<UpgradeAvailabilityChange>(stream, version)?;
            tech_availability_changes = read_count_vec::<TechAvailabilityChange>(stream, version)?;
            random_unit_tables = read_count_vec::<RandomUnitTable>(stream, version)?;
            if version > 24 {
                random_item_tables = read_count_vec::<RandomItemTable>(stream, version)?;
            }
        }

        Ok(Self {
            version,
            saves,
            editor_version,
            build_version,
            name,
            author,
            description,
            recommended_players,
            camera_bounds,
            camera_bounds_complements,
            playable_size,
            flags,
            tileset,
            campaign_background,
            loading_screen_model,
            loading_screen_text,
            loading_screen_title,
            loading_screen_subtitle,
            loading_screen,
            prologue_screen_model,
            prologue_screen_text,
            prologue_screen_title,
            prologue_screen_subtitle,
            use_terrain_fog,
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
}

impl War3MapW3i {
    /// Get the build version of the map as `major * 100 + minor` (e.g. 1.32 → 132)
    pub fn get_build_version(&self) -> u32 {
        match self.build_version {
            Some(version) => version[0] * 100 + version[1],
            None => 0,
        }
    }

    /// Collect TRIGSTR references from serialized map info → string table ids.
    ///
    /// Requires the `serde` feature.
    pub fn trigger_string_map(&self) -> Result<HashMap<String, i32>, ParserError> {
        #[cfg(feature = "serde")]
        {
            /// "TRIGSTR_007" / "TRIGSTR_007ab" / "TRIGSTR_7" / "TRIGSTR_-007"
            const TRIGGER_STR_RE: &str = r#""TRIGSTR_(-?\d+)(?:\w+)?""#;

            let re = regex::Regex::new(TRIGGER_STR_RE)?;
            let json = serde_json::to_string(&self)?;
            let mut trigger_strings = HashMap::new();
            for caps in re.captures_iter(json.as_str()) {
                let original = caps
                    .get(0)
                    .ok_or_else(|| ParserError::FailedToFindRegex(TRIGGER_STR_RE.to_string()))?
                    .as_str()
                    .to_string();
                if let Some(id) = caps.get(1) {
                    if let Ok(id) = id.as_str().parse::<i32>() {
                        trigger_strings.insert(original, id);
                    }
                }
            }
            Ok(trigger_strings)
        }

        #[cfg(not(feature = "serde"))]
        {
            let _ = self;
            Err(ParserError::FeatureRequired("serde"))
        }
    }
}

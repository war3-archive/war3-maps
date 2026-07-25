/* Hand-maintained TypeScript definitions for @wesleyel/war3parser.
 * Kept in sync with war3parser::model::MapSnapshot and related serde shapes.
 * Copied into dist/ by `just build-wasm` after wasm-pack.
 */

export interface War3MapHeader {
  has_hm3w: boolean;
  name: string | null;
  flags: number | null;
  max_players: number | null;
  u1: number | null;
}

export interface Player {
  id: number;
  player_type: number;
  race: number;
  is_fixed_start_position: number;
  name: string;
  start_location: number[];
  ally_low_priorities: number;
  ally_high_priorities: number;
  enemy_low_priorities: number | null;
  enemy_high_priorities: number | null;
}

export interface Force {
  flags: number;
  player_masks: number;
  name: string;
}

export interface UpgradeAvailabilityChange {
  player_flags: number;
  id: number[];
  level_affected: number;
  availability: number;
}

export interface TechAvailabilityChange {
  player_flags: number;
  id: number[];
}

export interface RandomUnit {
  chance: number;
  ids: number[][];
}

export interface RandomUnitTable {
  id: number;
  name: string;
  columns: number;
  column_types: number[];
  units: RandomUnit[];
}

export interface RandomItem {
  chance: number;
  id: number[];
}

export interface RandomItemSet {
  items: RandomItem[];
}

export interface RandomItemTable {
  id: number;
  name: string;
  sets: RandomItemSet[];
}

/** Parsed `war3map.w3i` (TRIGSTR-resolved when a string table was applied). */
export interface War3MapW3i {
  version: number;
  saves: number;
  editor_version: number;
  build_version: number[] | null;
  name: string;
  author: string;
  description: string;
  recommended_players: string;
  camera_bounds: number[];
  camera_bounds_complements: number[];
  playable_size: number[];
  flags: number;
  tileset: number;
  campaign_background: number;
  loading_screen_model: string | null;
  loading_screen_text: string;
  loading_screen_title: string;
  loading_screen_subtitle: string;
  loading_screen: number;
  prologue_screen_model: string | null;
  prologue_screen_text: string;
  prologue_screen_title: string;
  prologue_screen_subtitle: string;
  use_terrain_fog: number | null;
  fog_height: number[] | null;
  fog_density: number | null;
  fog_color: number[] | null;
  global_weather: number | null;
  sound_environment: string | null;
  light_environment_tileset: number | null;
  water_vertex_color: number[] | null;
  script_mode: number | null;
  graphics_mode: number | null;
  game_data_version: number | null;
  default_camera_zoom: number | null;
  max_camera_zoom: number | null;
  min_camera_zoom: number | null;
  skipped_optional_sections: boolean;
  players: Player[];
  forces: Force[];
  upgrade_availability_changes: UpgradeAvailabilityChange[];
  tech_availability_changes: TechAvailabilityChange[];
  random_unit_tables: RandomUnitTable[];
  random_item_tables: RandomItemTable[];
}

/** PNG data-URL image (minimap / preview). */
export interface War3ImageData {
  data_url: string;
  width: number;
  height: number;
  filename: string;
}

/** @deprecated alias of War3ImageData */
export type War3Image = War3ImageData;

export interface StringTableEntry {
  id: number;
  value: string;
}

export interface ImportEntry {
  path: string;
  is_custom: number;
}

/** `war3map.mmp` icon — coords on 256×256 minimap; type 0 mine / 1 house / 2 start */
export interface MinimapIcon {
  icon_type: number;
  x: number;
  y: number;
  /** RGBA */
  color: number[];
}

/**
 * Portable parse result (Rust: `MapSnapshot`).
 * Returned by {@link parse_map} as a plain JS object.
 */
export interface War3MapMetadata {
  header: War3MapHeader;
  map_info: War3MapW3i | null;
  images: War3ImageData[];
  minimap_icons: MinimapIcon[];
  imports: ImportEntry[] | null;
  strings: StringTableEntry[] | null;
  files: string[] | null;
  /** Wall-clock milliseconds spent parsing (set by the WASM binding). */
  parse_ms: number | null;
}

/** Alias matching the Rust type name. */
export type MapSnapshot = War3MapMetadata;

export default function init(
  module_or_path?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module,
): Promise<unknown>;

/** Parse a `.w3x` / `.w3m` buffer. Returns `undefined` if not a readable map. */
export function parse_map(buffer: Uint8Array): War3MapMetadata | undefined;

/** Backward-compatible alias of {@link parse_map}. */
export function get_map_info(buffer: Uint8Array): War3MapMetadata | undefined;

/** Crate version string. */
export function version(): string;

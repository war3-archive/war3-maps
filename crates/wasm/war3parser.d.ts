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

/** Unknown legacy fields only present in w3i format v8 (RoC beta). */
export interface LegacyV8Fields {
  unk1: number;
  unk2: number;
  unk3: number;
  unk4: number;
  unk5: number;
  unk6: number;
  unk7: number;
}

/**
 * Parsed `war3map.w3i` (TRIGSTR-resolved when a string table was applied).
 *
 * Supports format versions v8 - v33; fields marked with a version range are
 * `null` outside it.
 */
export interface War3MapW3i {
  version: number;
  /** v18+ */
  saves: number | null;
  /** v18+ */
  editor_version: number | null;
  /** `[major, minor, patch, build]`, v27+ */
  build_version: number[] | null;
  name: string;
  author: string;
  description: string;
  recommended_players: string;
  /** v8 only */
  legacy_v8: LegacyV8Fields | null;
  camera_bounds: number[];
  /** v15+ */
  camera_bounds_complements: number[] | null;
  playable_size: number[];
  flags: number;
  tileset: number;
  /** v23+ (was `campaign_background` before v23) */
  loading_screen_background: number | null;
  /** v18-v22 only */
  campaign_background: number | null;
  /** v15-v17, v23+ */
  loading_screen_model: string | null;
  /** v10+ */
  loading_screen_text: string | null;
  /** v10+ */
  loading_screen_title: string | null;
  /** v15+ */
  loading_screen_subtitle: string | null;
  /** 0 = default, 1 = custom, 2 = melee; v23+ */
  game_data_set: number | null;
  /** v18-v22 only */
  loading_screen_index: number | null;
  /** v15-v17, v23+ */
  prologue_screen_model: string | null;
  /** v11+ */
  prologue_screen_text: string | null;
  /** v11+ */
  prologue_screen_title: string | null;
  /** v15+ */
  prologue_screen_subtitle: string | null;
  /** v23+ */
  fog_style: number | null;
  /** `[startZ, endZ]`, v23+ */
  fog_height: number[] | null;
  /** v23+ */
  fog_density: number | null;
  /** BGRA, v23+ */
  fog_color: number[] | null;
  /** v25+ */
  global_weather: number | null;
  /** v23+ */
  sound_environment: string | null;
  /** v23+ */
  light_environment_tileset: number | null;
  /** BGRA, v23+ */
  water_vertex_color: number[] | null;
  /** 0 = JASS, 1 = Lua; v28+ */
  script_mode: number | null;
  /** 1 = SD, 2 = HD, 3 = both; v31+ */
  graphics_mode: number | null;
  /** 0 = ROC, 1 = TFT; v31+ */
  game_data_version: number | null;
  /** v32+ */
  default_camera_zoom: number | null;
  /** v32+ */
  max_camera_zoom: number | null;
  /** v33+ */
  min_camera_zoom: number | null;
  skipped_optional_sections: boolean;
  players: Player[];
  forces: Force[];
  upgrade_availability_changes: UpgradeAvailabilityChange[];
  tech_availability_changes: TechAvailabilityChange[];
  /** v15+ */
  random_unit_tables: RandomUnitTable[];
  /** v24+ */
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
  /**
   * Third-party modification found in the map script, when one is recognised.
   * Absent also covers "script unreadable", so it is not a clean bill of health.
   */
  modification?: ModInfo | null;
  /** Wall-clock milliseconds spent parsing (set by the WASM binding). */
  parse_ms: number | null;
}

/** A modification recognised inside a map script (Rust: `ModInfo`). */
export interface ModInfo {
  /** Stable identifier of the tool, e.g. `"hke"`. */
  tool: string;
  /** Human-readable name. */
  label: string;
  /** Build string taken from the injected banner, when recognised. */
  variant?: string | null;
  /** How a player triggers the injected menu, in the tool's own terms. */
  activation: string[];
  /** Which literals matched, so a result can be audited without a rescan. */
  evidence: string[];
  /** Where the tool documents itself. */
  reference: string;
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

/**
 * Parse a map, keeping the reason when it fails.
 * {@link parse_map} collapses every failure into `undefined`; this reports why.
 */
export function parse_map_result(buffer: Uint8Array): ParseResult;

/** Result of {@link parse_map_result}. */
export type ParseResult =
  | { ok: true; map: War3MapMetadata }
  | { ok: false; error: string };

/**
 * The archive's `(listfile)` entries.
 * `undefined` when the archive cannot be opened, `null` when it carries no
 * listfile — common for protected maps, whose files stay reachable by name.
 */
export function list_files(buffer: Uint8Array): string[] | null | undefined;

/**
 * Extract one file by its in-archive name, e.g. `war3map.j` or
 * `scripts\\war3map.j`. `undefined` when absent or unreadable.
 */
export function extract_file(buffer: Uint8Array, name: string): Uint8Array | undefined;

/**
 * Scan the map script for known third-party modifications.
 * `undefined` means no known signature matched — not that the map is clean.
 */
export function detect_modification(buffer: Uint8Array): ModInfo | undefined;

/** Crate version string. */
export function version(): string;

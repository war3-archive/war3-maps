/// <reference types="vite/client" />

declare module "@wesleyel/war3parser" {
  export interface War3MapHeader {
    has_hm3w: boolean;
    name?: string | null;
    flags?: number | null;
    max_players?: number | null;
    u1?: number | null;
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
    enemy_low_priorities?: number | null;
    enemy_high_priorities?: number | null;
  }

  export interface Force {
    flags: number;
    player_masks: number;
    name: string;
  }

  export interface War3MapW3i {
    version: number;
    saves: number;
    editor_version: number;
    build_version?: number[] | null;
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
    loading_screen_model?: string | null;
    loading_screen_text: string;
    loading_screen_title: string;
    loading_screen_subtitle: string;
    loading_screen: number;
    prologue_screen_model?: string | null;
    prologue_screen_text: string;
    prologue_screen_title: string;
    prologue_screen_subtitle: string;
    use_terrain_fog?: number | null;
    fog_height?: number[] | null;
    fog_density?: number | null;
    fog_color?: number[] | null;
    global_weather?: number | null;
    sound_environment?: string | null;
    light_environment_tileset?: number | null;
    water_vertex_color?: number[] | null;
    script_mode?: number | null;
    graphics_mode?: number | null;
    game_data_version?: number | null;
    default_camera_zoom?: number | null;
    max_camera_zoom?: number | null;
    min_camera_zoom?: number | null;
    skipped_optional_sections: boolean;
    players: Player[];
    forces: Force[];
  }

  export interface War3Image {
    data_url: string;
    width: number;
    height: number;
    filename: string;
  }

  export interface StringTableEntry {
    id: number;
    value: string;
  }

  export interface ImportEntry {
    path: string;
    is_custom: number;
  }

  /** war3map.mmp icon — coords on 256×256 minimap; type 0 mine / 1 house / 2 start */
  export interface MinimapIcon {
    icon_type: number;
    x: number;
    y: number;
    color: number[];
  }

  export interface War3MapMetadata {
    header: War3MapHeader;
    map_info?: War3MapW3i | null;
    images: War3Image[];
    minimap_icons: MinimapIcon[];
    imports?: ImportEntry[] | null;
    strings?: StringTableEntry[] | null;
    files?: string[] | null;
    parse_ms: number;
  }

  export default function init(module_or_path?: unknown): Promise<unknown>;
  export function parse_map(buffer: Uint8Array): War3MapMetadata | undefined;
  export function get_map_info(buffer: Uint8Array): War3MapMetadata | undefined;
  export function version(): string;
}

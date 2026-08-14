/** Human-friendly labels and flag decoding for w3i fields */

export const TILESETS: Record<string, string> = {
  A: "Ashenvale",
  B: "Barrens",
  C: "Felwood",
  D: "Dungeon",
  F: "Lordaeron Fall",
  G: "Underground",
  L: "Lordaeron Summer",
  N: "Northrend",
  Q: "Village Fall",
  V: "Village",
  W: "Lordaeron Winter",
  X: "Dalaran",
  Y: "Cityscape",
  Z: "Sunken Ruins",
  I: "Icecrown",
  J: "Dalaran Ruins",
  O: "Outland",
  K: "Black Citadel",
};

export const PLAYER_TYPES: Record<number, string> = {
  1: "User",
  2: "Computer",
  3: "Neutral",
  4: "Rescuable",
};

export const RACES: Record<number, string> = {
  0: "Selectable",
  1: "Human",
  2: "Orc",
  3: "Undead",
  4: "Night Elf",
};

/** Classic WC3 slot colors (player id 0–23). */
export const SLOT_COLORS: string[] = [
  "#ff0303",
  "#0042ff",
  "#1ce6b9",
  "#540081",
  "#fffc00",
  "#fe8a0e",
  "#20c000",
  "#e55bb0",
  "#959697",
  "#7ebff1",
  "#106246",
  "#4e2a04",
  "#9b0000",
  "#0000c3",
  "#00eaff",
  "#be00fe",
  "#ebcd87",
  "#f8a48b",
  "#bfff80",
  "#dcb9eb",
  "#282828",
  "#ebf0ff",
  "#00781e",
  "#a46f33",
];

export function slotColor(playerId: number): string {
  return SLOT_COLORS[((playerId % SLOT_COLORS.length) + SLOT_COLORS.length) % SLOT_COLORS.length];
}

export function raceIcon(race: number): string {
  switch (race) {
    case 1:
      return "🛡";
    case 2:
      return "⚔";
    case 3:
      return "💀";
    case 4:
      return "🌙";
    default:
      return "◇";
  }
}

export function controllerLabel(playerType: number): string {
  return PLAYER_TYPES[playerType] ?? `Type ${playerType}`;
}

/** Players belonging to a force bitmask, sorted by id. */
export function playersInForce<T extends { id: number }>(mask: number, players: T[]): T[] {
  const m = mask >>> 0;
  return players
    .filter((p) => {
      const id = p.id >>> 0;
      if (id >= 32) return false;
      return ((m >>> id) & 1) === 1;
    })
    .sort((a, b) => a.id - b.id);
}

const MAP_FLAGS: Array<[number, string]> = [
  [0x0001, "Hide minimap in preview"],
  [0x0002, "Modify ally priorities"],
  [0x0004, "Melee map"],
  [0x0008, "Show non-default waves"],
  [0x0010, "Masked areas partially visible"],
  [0x0020, "Fixed player settings"],
  [0x0040, "Custom forces"],
  [0x0080, "Custom techtree"],
  [0x0100, "Custom abilities"],
  [0x0200, "Custom upgrades"],
  [0x0400, "Map properties menu opened"],
  [0x0800, "Show water waves on cliff shores"],
  [0x1000, "Show water waves on rolling shores"],
  [0x2000, "Has terrain fog"],
  [0x4000, "Requires expansion (TFT)"],
  [0x8000, "Item classification data used"],
  [0x10000, "Water tinting used"],
  [0x20000, "Accurate probability for calculations"],
  [0x40000, "Custom ability skin used"],
];

export function tilesetName(code: number | string | undefined): string {
  if (code === undefined || code === null) return "—";
  const letter = typeof code === "number" ? String.fromCharCode(code) : String(code);
  return TILESETS[letter] ? `${TILESETS[letter]} (${letter})` : letter;
}

export function raceName(r: number): string {
  return RACES[r] ?? `Race ${r}`;
}

export function decodeFlags(flags: number | undefined): string[] {
  if (flags === undefined || flags === null) return [];
  const out: string[] = [];
  let rest = flags >>> 0;
  for (const [bit, label] of MAP_FLAGS) {
    if ((rest & bit) === bit) {
      out.push(label);
      rest &= ~bit;
    }
  }
  if (rest) out.push(`Other 0x${rest.toString(16)}`);
  return out;
}

export function scriptModeName(mode: number | null | undefined): string {
  if (mode === 0) return "JASS";
  if (mode === 1) return "Lua";
  if (mode == null) return "—";
  return `Mode ${mode}`;
}

export function graphicsModeName(mode: number | null | undefined): string {
  if (mode === 1) return "SD";
  if (mode === 2) return "HD";
  if (mode === 3) return "SD + HD";
  if (mode == null) return "—";
  return `Mode ${mode}`;
}

export function gameDataVersionName(v: number | null | undefined): string {
  if (v === 0) return "ROC";
  if (v === 1) return "TFT";
  if (v == null) return "—";
  return `Data ${v}`;
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

export function formatBuild(build: number[] | null | undefined): string {
  if (!build || build.length < 2) return "—";
  return `${build[0]}.${build[1]}.${build[2] ?? 0}.${build[3] ?? 0}`;
}

export function stripColorCodes(s: string | null | undefined): string {
  if (!s) return "";
  return s.replace(/\|c[0-9a-fA-F]{8}/g, "").replace(/\|r/gi, "").replace(/\|n/gi, "\n");
}

/** Era label for a w3i format version (full ladder v8-v33). */
export function w3iEraName(v: number | null | undefined): string {
  if (v == null) return "";
  if (v < 18) return "RoC beta";
  if (v < 23) return "Reign of Chaos";
  if (v < 28) return "The Frozen Throne";
  if (v < 31) return "1.31";
  if (v < 32) return "Reforged";
  return "WC3 2.0";
}

/** Render a rawcode byte array (e.g. upgrade/tech id) as its 4CC string. */
export function fourCC(bytes: number[] | null | undefined): string {
  if (!bytes || bytes.length !== 4) return "????";
  return bytes
    .map((b) => (b >= 0x20 && b < 0x7f ? String.fromCharCode(b) : "�"))
    .join("");
}

const WEATHER_NAMES: Record<string, string> = {
  RAhr: "Rain (heavy)",
  RAlr: "Rain (light)",
  MEds: "Dungeon mist",
  FDbh: "Black fog (heavy)",
  FDbl: "Black fog (light)",
  FDgh: "Green fog (heavy)",
  FDgl: "Green fog (light)",
  FDrh: "Red fog (heavy)",
  FDrl: "Red fog (light)",
  FDwh: "White fog (heavy)",
  FDwl: "White fog (light)",
  SNbs: "Blizzard",
  SNhs: "Snow (heavy)",
  SNls: "Snow (light)",
  WOcw: "Wind (heavy)",
  WOlw: "Wind (light)",
  LRaa: "Ashenvale rain",
  LRma: "Moonlight ambience",
};

/** Global weather rawcode (i32, little-endian) → readable name. */
export function weatherName(code: number | null | undefined): string {
  if (code == null) return "—";
  if (code === 0) return "None";
  const cc = fourCC([code & 0xff, (code >> 8) & 0xff, (code >> 16) & 0xff, (code >> 24) & 0xff]);
  return WEATHER_NAMES[cc] ? `${WEATHER_NAMES[cc]} (${cc})` : cc;
}

export function gameDataSetName(v: number | null | undefined): string {
  if (v === 0) return "Default";
  if (v === 1) return "Custom";
  if (v === 2) return "Melee";
  if (v == null) return "—";
  return `Set ${v}`;
}

/** File-order BGRA byte array → CSS color. */
export function bgraToCss(bytes: number[] | null | undefined): string | null {
  if (!bytes || bytes.length !== 4) return null;
  const [b, g, r, a] = bytes;
  return `rgba(${r}, ${g}, ${b}, ${(a / 255).toFixed(2)})`;
}

/** Import flag byte → standard (implicit war3mapimported\) vs custom path. */
export function importFlagLabel(flag: number): string {
  if (flag === 0 || flag === 1 || flag === 8) return `standard (${flag})`;
  if (flag === 10 || flag === 13) return `custom (${flag})`;
  return `flag ${flag}`;
}

export function availabilityName(v: number): string {
  if (v === 0) return "Unavailable";
  if (v === 1) return "Available";
  if (v === 2) return "Researched";
  return `Mode ${v}`;
}

/** Bitmask of player slots → compact list like "P0, P1, P4" (or "all"). */
export function playerMaskLabel(mask: number): string {
  const m = mask >>> 0;
  if (m === 0xffffffff) return "All players";
  const slots: number[] = [];
  for (let i = 0; i < 32; i++) {
    if ((m >>> i) & 1) slots.push(i);
  }
  if (!slots.length) return "None";
  if (slots.length > 8) return `${slots.length} players`;
  return slots.map((i) => `P${i}`).join(", ");
}

export function extensionOf(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot >= 0 ? base.slice(dot).toLowerCase() : "";
}

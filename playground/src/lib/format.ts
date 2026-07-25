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

export function extensionOf(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot >= 0 ? base.slice(dot).toLowerCase() : "";
}

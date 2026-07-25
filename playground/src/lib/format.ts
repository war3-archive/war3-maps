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
  1: "Human",
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

/** Common war3map.w3i map flags (bit → label). Unknown bits still shown as hex. */
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

export function playerTypeName(t: number): string {
  return PLAYER_TYPES[t] ?? `Type ${t}`;
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

export function playersFromMask(mask: number, players: Array<{ id: number; name: string }>): string {
  const names: string[] = [];
  for (let i = 0; i < 28; i++) {
    if (mask & (1 << i)) {
      const p = players.find((x) => x.id === i);
      names.push(p ? stripColorCodes(p.name) || `P${i}` : `P${i}`);
    }
  }
  return names.join(", ") || "—";
}

export function extensionOf(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  const dot = base.lastIndexOf(".");
  return dot >= 0 ? base.slice(dot).toLowerCase() : "";
}

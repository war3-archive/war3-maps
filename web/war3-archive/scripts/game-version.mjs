// Derives "this map needs at least client version X" from the w3i.
//
// The claim is deliberately a FLOOR, not a match: a wrong floor that is too low
// is merely uninformative, a wrong floor that is too high tells someone their
// working map won't run. So every rule here may only raise the floor, and a
// signal we cannot map is ignored rather than guessed at.
//
// Evidence, most reliable first:
//   1. build_version — the [major, minor, revision, build] quadruple present
//      from w3i v27 onward. Exact; nothing is inferred.
//   2. format_version — the w3i format ladder. Coarse but certain: a v25 file
//      cannot be opened by a pre-TFT client, a v28 file needs 1.31+.
//   3. editor_version — the World Editor build that last saved the map. The
//      only signal that separates patches *inside* the huge v25 bucket, but it
//      needs a build->patch table that is community knowledge rather than
//      anything derivable from the files themselves. Only entries we are
//      reasonably confident about appear below; everything else falls through
//      to the format floor on purpose.
//
// Note on the "1.24 boundary" the community talks about: 1.24 fixed the JASS
// return bug and broke a lot of older maps. That is a runtime behaviour change,
// not a format change — a 1.20 map and a 1.24 map are both w3i v25 and are
// indistinguishable here. Detecting it needs script-level analysis of
// war3map.j, so this module deliberately makes no claim about it.

/** Client families, used to pick the badge glyph. */
export const CLIENTS = ["roc", "tft", "reforged"];

// w3i format version -> [client, floor label]. These are format guarantees.
const FORMAT_FLOOR = new Map([
  [8, ["roc", "1.00"]], // RoC betas
  [15, ["roc", "1.00"]],
  [18, ["roc", "1.00"]], // RoC release
  [25, ["tft", "1.07"]], // TFT — spans 1.07 through 1.28
  [26, ["tft", "1.29"]],
  [27, ["tft", "1.29"]],
  [28, ["reforged", "1.31"]],
  [31, ["reforged", "1.32"]],
  [32, ["reforged", "1.33"]],
  [33, ["reforged", "2.0"]],
]);

// World Editor build -> patch that build shipped with. Community-sourced; only
// values we would defend are listed. Unlisted builds (including the 6030-6058
// range that covers most of this archive) intentionally do not raise anything.
//
// 6115 is the one entry calibrated against this corpus: the only two maps
// carrying an exact build_version report 1.35 alongside editor build 6115.
const EDITOR_FLOOR = new Map([
  [6059, "1.24"],
  [6060, "1.26"],
  [6061, "1.26"],
  [6072, "1.27"],
  [6115, "1.35"],
]);

// Builds outside any plausible range (corrupt fields, protection tooling).
const isSaneEditorBuild = (value) =>
  Number.isInteger(value) && value >= 4000 && value <= 20000;

function compareVersions(a, b) {
  const left = String(a).split(".").map(Number);
  const right = String(b).split(".").map(Number);
  for (let index = 0; index < Math.max(left.length, right.length); index += 1) {
    const difference = (left[index] || 0) - (right[index] || 0);
    if (difference !== 0) return difference;
  }
  return 0;
}

/**
 * @returns {{client: string, version: string|null, exact: boolean, evidence: string}}
 */
export function minimumVersion(record) {
  const extension = String(record.extension || "").toLowerCase();
  const formatVersion = record.format_version ?? null;
  const editorVersion = record.editor_version ?? null;
  const buildVersion = record.build_version ?? null;

  // 1. Exact, when the file states it outright.
  if (Array.isArray(buildVersion) && buildVersion.length >= 2) {
    const [major, minor, revision] = buildVersion;
    const version = revision ? `${major}.${minor}.${revision}` : `${major}.${minor}`;
    return {
      client: major >= 1 && minor >= 31 ? "reforged" : "tft",
      version,
      exact: true,
      evidence: `w3i 内记录的游戏版本 ${buildVersion.join(".")}`,
    };
  }

  // 2. Format floor. .w3m is RoC-only, .w3n campaigns are TFT-only.
  const floor = FORMAT_FLOOR.get(formatVersion);
  let client = floor?.[0] ?? (extension === "w3m" ? "roc" : "tft");
  let version = floor?.[1] ?? null;
  const evidence = [];
  if (floor) evidence.push(`w3i 格式 v${formatVersion}`);
  else if (formatVersion === null) evidence.push("w3i 不可读");

  // 3. Editor build, only ever upward.
  if (isSaneEditorBuild(editorVersion)) {
    const raised = EDITOR_FLOOR.get(editorVersion);
    if (raised && (version === null || compareVersions(raised, version) > 0)) {
      version = raised;
      if (client === "roc") client = "tft";
    }
    evidence.push(`编辑器 build ${editorVersion}`);
  }

  return { client, version, exact: false, evidence: evidence.join(" · ") };
}

/** Display label: "1.24+" for a floor, "1.35" when exact, null when unknown. */
export function versionLabel(result) {
  if (!result.version) return null;
  return result.exact ? result.version : `${result.version}+`;
}

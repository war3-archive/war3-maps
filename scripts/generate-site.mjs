// Turns the Hugging Face catalog into the static JSON the site fetches at runtime.
//
// Nothing image-shaped is produced here: covers live in the dataset and are
// hotlinked, so the Pages artifact stays around 20 MB instead of 250 MB and the
// build no longer re-encodes ~9200 thumbnails on every run.
//
// Records are emitted as tuples rather than objects. Repeating twenty key names
// across 10k entries costs more than the values themselves, and everything
// derivable from the SHA-256 (download URL, cover URL) is derived on the client.

import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { CLIENTS, minimumVersion, versionLabel } from "./game-version.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const catalogUrl =
  process.env.HF_CATALOG_URL ||
  "https://huggingface.co/datasets/magicwenli/war3-maps/resolve/main/catalog/maps.json";

// Categories are paged so a browser never pulls 2781 records to show 48 of them.
const PAGE_SIZE = 500;

const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });

let payload;
if (process.env.HF_CATALOG_FILE) {
  payload = JSON.parse(await readFile(path.join(repoRoot, process.env.HF_CATALOG_FILE), "utf-8"));
} else {
  const response = await fetch(catalogUrl, {
    headers: { "User-Agent": "war3-maps-site-generator/2" },
  });
  if (!response.ok) {
    throw new Error(`catalog fetch failed: ${response.status} ${response.statusText}`);
  }
  payload = await response.json();
}
const maps = Array.isArray(payload) ? payload : payload.maps || [];
if (maps.length === 0) throw new Error("catalog contains no maps");

// Derive the dataset base URL from the catalog itself instead of hardcoding it,
// so pointing HF_CATALOG_URL at a fork keeps every link consistent.
function baseOf(sample, kind) {
  const url = String(sample || "");
  const marker = `/${kind}/`;
  const index = url.indexOf(marker);
  return index === -1 ? null : url.slice(0, index + 1);
}
const objectBase =
  baseOf(maps.find((item) => item.download_url)?.download_url, "objects") ??
  "https://huggingface.co/datasets/magicwenli/war3-maps/resolve/main/";
const coverBase =
  baseOf(maps.find((item) => item.cover_url)?.cover_url, "covers") ?? objectBase;

const dataDir = path.join(repoRoot, "public/data");
await rm(dataDir, { recursive: true, force: true });
await mkdir(path.join(dataDir, "categories"), { recursive: true });

function collectionOf(record) {
  return record.collection || record.category || "未分类";
}

function playersOf(record) {
  const value = record.player_count ?? record.max_players ?? null;
  return Number.isFinite(value) && value > 0 ? value : null;
}

// A filename that is just "<name>.<ext>" carries no extra search signal.
function distinctFilename(record) {
  const filename = record.filename || "";
  if (!filename) return null;
  return filename === `${record.name}.${record.extension}` ? null : filename;
}

const byCollection = new Map();
for (const record of maps) {
  const name = collectionOf(record);
  if (!byCollection.has(name)) byCollection.set(name, []);
  byCollection.get(name).push(record);
}

const collections = [...byCollection.keys()].sort(
  (a, b) => byCollection.get(b).length - byCollection.get(a).length,
);

// The minimum client version a map needs is derived once here rather than in
// the browser, so the inference rules live in one auditable place.
const versionOf = (record) => {
  const result = minimumVersion(record);
  return {
    client: CLIENTS.indexOf(result.client),
    label: versionLabel(result),
    evidence: result.evidence,
  };
};

// Card tuple: name, author, description, size, players, extension, client index,
// minimum-version label, evidence string, has cover.
const CARD_FIELDS = [
  "sha256", "name", "author", "description", "size", "players", "ext",
  "client", "min_version", "version_evidence", "cover",
];
const cardOf = (record) => {
  const version = versionOf(record);
  return [
    record.sha256,
    record.name || record.filename || "",
    record.author || "",
    record.description || "",
    record.size ?? 0,
    playersOf(record),
    record.extension || "",
    version.client,
    version.label,
    version.evidence,
    record.cover_path ? 1 : 0,
  ];
};

const overviewCollections = [];
// Position of each map inside its (name-sorted) category, so a search result can
// deep-link straight into the right shard instead of rendering everything before it.
const offsetOf = new Map();
let coverCount = 0;
for (const name of collections) {
  // Raw UTF-8 path segment: the browser percent-encodes the request and the
  // server decodes it back, so the directory on disk must NOT be pre-encoded.
  const slug = name;
  const list = byCollection
    .get(name)
    .slice()
    .sort((a, b) => collator.compare(a.name || a.filename || "", b.name || b.filename || ""));
  list.forEach((record, index) => offsetOf.set(record.sha256, index));
  const pageCount = Math.max(1, Math.ceil(list.length / PAGE_SIZE));
  await mkdir(path.join(dataDir, "categories", slug), { recursive: true });
  for (let page = 0; page < pageCount; page += 1) {
    const slice = list.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);
    await writeFile(
      path.join(dataDir, "categories", slug, `${page}.json`),
      JSON.stringify({
        collection: name,
        page,
        page_count: pageCount,
        total: list.length,
        fields: CARD_FIELDS,
        maps: slice.map(cardOf),
      }),
    );
  }
  coverCount += list.filter((record) => record.cover_path).length;
  overviewCollections.push({ name, slug, count: list.length, page_count: pageCount });
}

// Search tuple: name, author, filename (only when it differs), collection index,
// size, players, w3i version, extension, has cover.
const SEARCH_FIELDS = ["sha256", "name", "author", "filename", "collection", "size", "players", "min_version", "ext", "client", "offset"];
const collectionIndex = new Map(collections.map((name, index) => [name, index]));
const searchIndex = maps
  .slice()
  .sort((a, b) => collator.compare(a.name || a.filename || "", b.name || b.filename || ""))
  .map((record) => {
    const version = versionOf(record);
    return [
      record.sha256,
      record.name || record.filename || "",
      record.author || "",
      distinctFilename(record),
      collectionIndex.get(collectionOf(record)) ?? 0,
      record.size ?? 0,
      playersOf(record),
      version.label,
      record.extension || "",
      version.client,
      offsetOf.get(record.sha256) ?? 0,
    ];
  });

await writeFile(
  path.join(dataDir, "search-index.json"),
  JSON.stringify({
    fields: SEARCH_FIELDS,
    collections: collections.map((name) => ({ name, slug: name })),
    maps: searchIndex,
  }),
);

const overview = {
  updated_at: payload.generated_at_unix ? payload.generated_at_unix * 1000 : null,
  map_count: payload.map_count ?? maps.length,
  playable_map_count: payload.playable_map_count ?? null,
  campaign_count: payload.campaign_count ?? null,
  total_bytes: payload.total_bytes ?? null,
  cover_count: coverCount,
  page_size: PAGE_SIZE,
  object_base: objectBase,
  cover_base: coverBase,
  dataset_url: process.env.HF_DATASET_REPO
    ? `https://huggingface.co/datasets/${process.env.HF_DATASET_REPO}`
    : objectBase.split("/resolve/")[0] || null,
  collections: overviewCollections,
};
await writeFile(path.join(dataDir, "overview.json"), JSON.stringify(overview));

// Consumed at build time by index.astro and getStaticPaths — the pages ship a
// shell plus these counts, never the cards themselves.
await mkdir(path.join(repoRoot, "src/data"), { recursive: true });
await writeFile(path.join(repoRoot, "src/data/build-info.json"), JSON.stringify(overview, null, 2));

const bytes = (await readFile(path.join(dataDir, "search-index.json"))).length;
console.log(
  `generated ${maps.length} maps, ${coverCount} covers, ${collections.length} categories, ` +
    `search index ${(bytes / 1024 / 1024).toFixed(2)} MB`,
);

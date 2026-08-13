// Shared rendering helpers for the category and search views.
//
// Records arrive as tuples (see scripts/generate-site.mjs). Download and cover
// URLs are rebuilt here from the SHA-256 rather than shipped per record, which
// is what keeps the payloads small.

import buildInfo from "../data/build-info.json";

export const BASE = import.meta.env.BASE_URL;
/** Cards appended per "显示更多" click. */
export const PAGE_SIZE = 48;
/** Records per category shard file, as written by scripts/generate-site.mjs. */
export const SHARD_SIZE = buildInfo.page_size;
export const collections = buildInfo.collections;

const OBJECT_BASE = buildInfo.object_base;
const COVER_BASE = buildInfo.cover_base;

export const PLACEHOLDER_COVER = `data:image/svg+xml;utf8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512"><rect width="100%" height="100%" fill="#1c231d"/><text x="50%" y="52%" fill="#6f796f" font-family="sans-serif" font-size="40" text-anchor="middle">暂无预览图</text></svg>`,
)}`;

export const normalize = (value) => String(value ?? "").normalize("NFKC").toLocaleLowerCase("zh-CN");
export const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });

export function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (!Number.isFinite(value) || value <= 0) return "未知";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** exponent).toFixed(exponent > 1 ? 1 : 0)} ${units[exponent]}`;
}

export function downloadUrl(sha256, extension) {
  if (!sha256 || !extension) return null;
  return `${OBJECT_BASE}objects/${sha256.slice(0, 2)}/${sha256}.${extension}?download=true`;
}

export function coverUrl(sha256) {
  return `${COVER_BASE}covers/${sha256.slice(0, 2)}/${sha256}.webp`;
}

export async function fetchJSON(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

export const anchorOf = (sha256) => `m-${String(sha256).slice(0, 12)}`;

// Client badges. Deliberately abstract marks rather than Blizzard artwork:
// an orb for Reign of Chaos, a frozen shard for The Frozen Throne, a shield for
// Reforged and later.
const CLIENT_GLYPHS = [
  {
    name: "混乱之治",
    short: "RoC",
    color: "#b8763a",
    path: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 4.2a4.8 4.8 0 1 1 0 9.6 4.8 4.8 0 0 1 0-9.6Z",
  },
  {
    name: "冰封王座",
    short: "TFT",
    color: "#69a8c8",
    path: "M12 2 14.4 8.2 21 9l-4.8 4.3L17.6 21 12 17.4 6.4 21l1.4-7.7L3 9l6.6-.8Z",
  },
  {
    name: "重制版",
    short: "REF",
    color: "#d5aa50",
    path: "M12 2.5 20 5.4v6.2c0 4.5-3.2 8.4-8 9.9-4.8-1.5-8-5.4-8-9.9V5.4Zm0 3.4L7 7.7v4c0 2.7 1.9 5.1 5 6.3 3.1-1.2 5-3.6 5-6.3v-4Z",
  },
];

function clientBadge(clientIndex, label, evidence) {
  const glyph = CLIENT_GLYPHS[clientIndex] ?? CLIENT_GLYPHS[1];
  const wrap = document.createElement("span");
  wrap.className = "client-badge";
  const title = [glyph.name, label ? `最低需要 ${label}` : "版本未知", evidence]
    .filter(Boolean)
    .join(" · ");
  wrap.title = title;

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", glyph.path);
  path.setAttribute("fill", glyph.color);
  svg.append(path);

  const text = document.createElement("span");
  text.textContent = label ?? "未知";

  wrap.append(svg, text);
  // Screen readers get the client name; the glyph alone would say nothing.
  const sr = document.createElement("span");
  sr.className = "sr-only";
  sr.textContent = `（${glyph.name}）`;
  wrap.append(sr);
  return wrap;
}

function metadataRow(term, value) {
  const group = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = term;
  const dd = document.createElement("dd");
  dd.textContent = value || "未知";
  dd.title = dd.textContent;
  group.append(dt, dd);
  return group;
}

/**
 * Build a card from a category tuple:
 * [sha, name, author, desc, size, players, ext, client, minVersion, evidence, cover].
 */
export function buildCard(tuple, collectionName) {
  const [sha256, name, author, description, size, players, extension, client, minVersion, evidence, hasCover] =
    tuple;
  const label = name || "地图";

  const article = document.createElement("article");
  article.className = "map-card";
  article.id = anchorOf(sha256);

  const coverWrap = document.createElement("div");
  coverWrap.className = "cover-wrap";
  const cover = document.createElement("img");
  cover.className = "cover";
  cover.loading = "lazy";
  cover.decoding = "async";
  cover.width = 256;
  cover.height = 256;
  if (hasCover) {
    cover.src = coverUrl(sha256);
    cover.alt = `${label} 封面`;
    cover.addEventListener(
      "error",
      () => {
        cover.src = PLACEHOLDER_COVER;
      },
      { once: true },
    );
  } else {
    cover.src = PLACEHOLDER_COVER;
    cover.alt = `${label}（暂无预览图）`;
  }
  coverWrap.append(cover);

  const top = document.createElement("div");
  top.className = "card-topline";
  const category = document.createElement("span");
  category.className = "category";
  category.textContent = collectionName || "未分类";
  const sizeLabel = document.createElement("span");
  sizeLabel.className = "size";
  sizeLabel.textContent = formatBytes(size);
  top.append(category, sizeLabel);

  const heading = document.createElement("h2");
  heading.className = "name";
  heading.textContent = label;

  const summary = document.createElement("p");
  summary.className = "description";
  summary.textContent = description || "暂无简介";

  const metadata = document.createElement("dl");
  metadata.className = "metadata";
  const versionCell = document.createElement("div");
  const versionTerm = document.createElement("dt");
  versionTerm.textContent = "最低版本";
  const versionValue = document.createElement("dd");
  versionValue.append(clientBadge(client, minVersion, evidence));
  versionCell.append(versionTerm, versionValue);
  metadata.append(
    metadataRow("作者", author),
    metadataRow("玩家", players ? `${players} 人` : ""),
    metadataRow("格式", extension),
    versionCell,
  );

  const actions = document.createElement("div");
  actions.className = "card-actions";
  const download = document.createElement("a");
  download.className = "download";
  const href = downloadUrl(sha256, extension);
  if (href) {
    download.href = href;
    download.setAttribute("download", "");
    download.textContent = "单图下载";
  } else {
    download.setAttribute("aria-disabled", "true");
    download.textContent = "暂不可下载";
  }
  const copyHash = document.createElement("button");
  copyHash.type = "button";
  copyHash.className = "copy-hash";
  copyHash.dataset.sha = sha256;
  copyHash.textContent = "复制 SHA-256";
  actions.append(download, copyHash);

  article.append(coverWrap, top, heading, summary, metadata, actions);
  return article;
}

/** Delegated handler for the copy-hash buttons inside a results container. */
export function attachCopyHash(container) {
  container.addEventListener("click", async (event) => {
    const button = event.target.closest(".copy-hash");
    if (!button) return;
    try {
      await navigator.clipboard.writeText(button.dataset.sha || "");
      button.textContent = "已复制";
    } catch {
      button.textContent = "复制失败";
    }
    setTimeout(() => {
      button.textContent = "复制 SHA-256";
    }, 1400);
  });
}

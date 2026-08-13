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

/** Build a card from a category tuple: [sha, name, author, desc, size, players, ext, ver, cover]. */
export function buildCard(tuple, collectionName) {
  const [sha256, name, author, description, size, players, extension, version, hasCover] = tuple;
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
  metadata.append(
    metadataRow("作者", author),
    metadataRow("玩家", players ? `${players} 人` : ""),
    metadataRow("格式", extension),
    metadataRow("版本", version ? `w3i v${version}` : ""),
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

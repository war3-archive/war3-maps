// Home page search. The index is fetched once, normalized once, and then
// queried with plain substring tests — the previous version re-normalized all
// 10365 records on every keystroke, which dominated the typing latency.

import {
  BASE,
  PAGE_SIZE,
  collator,
  downloadUrl,
  fetchJSON,
  formatBytes,
  normalize,
} from "./map-browser.js";

const SHA = 0;
const NAME = 1;
const AUTHOR = 2;
const FILENAME = 3;
const COLLECTION = 4;
const SIZE = 5;
const PLAYERS = 6;
const MIN_VERSION = 7;
const EXT = 8;
const CLIENT = 9;
const OFFSET = 10;

const CLIENT_NAMES = ["混乱之治", "冰封王座", "重制版"];

const state = {
  maps: null,
  collections: [],
  haystack: null,
  query: "",
  sort: "name",
  filtered: [],
  shown: PAGE_SIZE,
};

const input = document.querySelector("#search-input");
const notice = document.querySelector("#notice");
const results = document.querySelector("#results");
const more = document.querySelector("#load-more");

async function ensureIndex() {
  if (state.maps) return;
  const index = await fetchJSON(`${BASE}data/search-index.json`);
  state.maps = index.maps;
  state.collections = index.collections;
  // One pass up front; every later query is a substring test against this.
  state.haystack = index.maps.map((entry) =>
    normalize(
      `${entry[NAME]} ${entry[AUTHOR]} ${entry[FILENAME] || ""} ${
        index.collections[entry[COLLECTION]]?.name || ""
      } ${CLIENT_NAMES[entry[CLIENT]] || ""} ${entry[MIN_VERSION] || ""}`,
    ),
  );
}

function resultRow(entry) {
  const collection = state.collections[entry[COLLECTION]];
  const row = document.createElement("div");
  row.className = "result-row";

  const main = document.createElement("div");
  main.className = "result-main";
  const name = document.createElement("a");
  name.className = "result-name";
  // ?i= lands the category view directly on this map's shard.
  name.href = `${BASE}categories/${collection.slug}/?i=${entry[OFFSET]}#m-${String(entry[SHA]).slice(0, 12)}`;
  name.textContent = entry[NAME] || entry[FILENAME] || "未知";
  const meta = document.createElement("p");
  meta.className = "result-meta";
  const parts = [entry[AUTHOR] || "未知作者", collection.name];
  if (entry[PLAYERS]) parts.push(`${entry[PLAYERS]} 人`);
  if (entry[MIN_VERSION]) {
    parts.push(`${CLIENT_NAMES[entry[CLIENT]] ?? ""} ${entry[MIN_VERSION]}`.trim());
  }
  parts.push(formatBytes(entry[SIZE]));
  meta.textContent = parts.join(" · ");
  main.append(name, meta);

  const download = document.createElement("a");
  download.className = "download result-download";
  const href = downloadUrl(entry[SHA], entry[EXT]);
  if (href) {
    download.href = href;
    download.setAttribute("download", "");
    download.textContent = "单图下载";
  } else {
    download.setAttribute("aria-disabled", "true");
    download.textContent = "暂不可下载";
  }

  row.append(main, download);
  return row;
}

function render() {
  const visible = state.filtered.slice(0, state.shown);
  const fragment = document.createDocumentFragment();
  for (const entry of visible) fragment.append(resultRow(entry));
  results.replaceChildren(fragment);

  notice.hidden = state.filtered.length > 0;
  if (state.filtered.length === 0) {
    notice.textContent = state.query
      ? "没有匹配的地图，试试减少关键词或切换分类。"
      : "输入关键词开始搜索，或选择一个分类浏览。";
  }
  more.hidden = state.filtered.length === 0 || state.shown >= state.filtered.length;
  more.textContent = `显示更多（${visible.length} / ${state.filtered.length}）`;
}

function sortFiltered() {
  if (state.sort === "size-desc") {
    state.filtered.sort((a, b) => Number(b[SIZE] || 0) - Number(a[SIZE] || 0));
  } else if (state.sort === "players-desc") {
    state.filtered.sort((a, b) => Number(b[PLAYERS] || 0) - Number(a[PLAYERS] || 0));
  } else {
    // The index is already name-sorted, so restore that order cheaply.
    state.filtered.sort((a, b) => collator.compare(a[NAME] || "", b[NAME] || ""));
  }
}

async function applySearch() {
  const terms = normalize(state.query).split(/\s+/).filter(Boolean);
  if (terms.length === 0) {
    state.filtered = [];
    state.shown = PAGE_SIZE;
    render();
    return;
  }
  await ensureIndex();
  const { maps, haystack } = state;
  const filtered = [];
  for (let index = 0; index < maps.length; index += 1) {
    const text = haystack[index];
    let match = true;
    for (const term of terms) {
      if (!text.includes(term)) {
        match = false;
        break;
      }
    }
    if (match) filtered.push(maps[index]);
  }
  state.filtered = filtered;
  sortFiltered();
  state.shown = PAGE_SIZE;
  render();
}

function runSearch() {
  applySearch().catch((error) => {
    notice.hidden = false;
    notice.textContent = `搜索失败：${error.message}`;
  });
}

let debounce;
input.addEventListener("input", (event) => {
  state.query = event.target.value;
  clearTimeout(debounce);
  debounce = setTimeout(runSearch, 150);
});

document.querySelector("#search-form").addEventListener("submit", (event) => {
  event.preventDefault();
  clearTimeout(debounce);
  runSearch();
});

document.querySelector("#sort-select").addEventListener("change", (event) => {
  state.sort = event.target.value;
  sortFiltered();
  state.shown = PAGE_SIZE;
  render();
});

more.addEventListener("click", () => {
  state.shown += PAGE_SIZE;
  render();
});

// Warm the index while the user is still reading the page.
if ("requestIdleCallback" in window) {
  requestIdleCallback(() => ensureIndex().catch(() => {}), { timeout: 3000 });
}

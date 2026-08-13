const PAGE_SIZE = 48;
const collator = new Intl.Collator("zh-CN", { numeric: true, sensitivity: "base" });
const state = { maps: [], filtered: [], query: "", category: "全部", sort: "name", shown: PAGE_SIZE };
const PLACEHOLDER_COVER = `data:image/svg+xml;utf8,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="100%" height="100%" fill="#1c231d"/><text x="50%" y="50%" fill="#6f796f" font-family="sans-serif" font-size="30" text-anchor="middle">暂无预览图</text></svg>`
)}`;

const $ = (selector) => document.querySelector(selector);
const normalize = (value) => String(value ?? "").normalize("NFKC").toLocaleLowerCase("zh-CN");
const text = (node, value, fallback = "未知") => { node.textContent = value || fallback; };

function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (!Number.isFinite(value) || value <= 0) return "未知";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** exponent).toFixed(exponent > 1 ? 1 : 0)} ${units[exponent]}`;
}

function collectionOf(map) {
  return map.collection || map.category || "未分类";
}

function searchable(map) {
  return normalize([map.name, map.author, map.description, map.collection, ...(map.collections || []), map.category, map.filename, ...(map.aliases || [])].join(" "));
}

function applyFilters() {
  const terms = normalize(state.query).split(/\s+/).filter(Boolean);
  state.filtered = state.maps.filter((map) => {
    if (state.category !== "全部" && collectionOf(map) !== state.category) return false;
    const haystack = map._search || (map._search = searchable(map));
    return terms.every((term) => haystack.includes(term));
  });
  state.filtered.sort((a, b) => {
    if (state.sort === "size-desc") return Number(b.size || 0) - Number(a.size || 0);
    if (state.sort === "players-desc") return Number(b.max_players || b.recommended_players || 0) - Number(a.max_players || a.recommended_players || 0);
    return collator.compare(a.name || a.filename || "", b.name || b.filename || "");
  });
  state.shown = PAGE_SIZE;
  render();
}

function render() {
  const results = $("#results");
  results.replaceChildren();
  const visible = state.filtered.slice(0, state.shown);
  for (const map of visible) {
    const fragment = $("#map-template").content.cloneNode(true);
    text(fragment.querySelector(".category"), collectionOf(map), "未分类");
    text(fragment.querySelector(".size"), formatBytes(map.size));
    text(fragment.querySelector(".name"), map.name || map.filename);
    text(fragment.querySelector(".description"), map.description, "暂无简介");
    text(fragment.querySelector(".author"), map.author);
    text(fragment.querySelector(".players"), map.player_count || map.max_players || map.recommended_players);
    text(fragment.querySelector(".format"), map.extension || map.format_version);
    text(fragment.querySelector(".version"), map.format_version ? `w3i v${map.format_version}` : "未知");
    const cover = fragment.querySelector(".cover");
    const coverName = map.name || map.filename || "地图";
    if (map.cover_data) {
      cover.src = map.cover_data;
      cover.alt = `${coverName} 封面`;
      cover.onerror = () => { cover.src = PLACEHOLDER_COVER; };
    } else {
      cover.src = PLACEHOLDER_COVER;
      cover.alt = `${coverName}（暂无预览图）`;
    }
    const download = fragment.querySelector(".download");
    if (map.download_url) {
      download.href = map.download_url;
      download.setAttribute("download", map.filename || "");
    } else {
      download.removeAttribute("href");
      download.textContent = "暂不可下载";
      download.setAttribute("aria-disabled", "true");
    }
    fragment.querySelector(".copy-hash").addEventListener("click", async (event) => {
      await navigator.clipboard.writeText(map.sha256 || "");
      event.currentTarget.textContent = "已复制";
      setTimeout(() => { event.currentTarget.textContent = "复制 SHA-256"; }, 1400);
    });
    results.append(fragment);
  }
  const notice = $("#notice");
  notice.hidden = state.maps.length > 0;
  if (state.maps.length > 0 && state.filtered.length === 0) {
    notice.hidden = false;
    notice.textContent = "没有匹配的地图，试试减少关键词或切换分类。";
  }
  const more = $("#load-more");
  more.hidden = state.shown >= state.filtered.length;
  more.textContent = `显示更多（${visible.length} / ${state.filtered.length}）`;
}

function buildCategories() {
  const counts = new Map([["全部", state.maps.length]]);
  for (const map of state.maps) counts.set(collectionOf(map), (counts.get(collectionOf(map)) || 0) + 1);
  const target = $("#category-list");
  target.replaceChildren();
  for (const [category, count] of [...counts].sort((a, b) => b[1] - a[1])) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = `${category} ${count}`;
    button.setAttribute("aria-pressed", String(category === state.category));
    button.addEventListener("click", () => {
      state.category = category;
      for (const item of target.children) item.setAttribute("aria-pressed", String(item === button));
      applyFilters();
    });
    target.append(button);
  }
}

async function start() {
  try {
    const [catalogResponse, configResponse] = await Promise.all([
      fetch("./data/maps.json"),
      fetch("./data/site-config.json").catch(() => null),
    ]);
    if (!catalogResponse.ok) throw new Error(`HTTP ${catalogResponse.status}`);
    const payload = await catalogResponse.json();
    state.maps = Array.isArray(payload) ? payload : (payload.maps || []);
    const config = configResponse?.ok ? await configResponse.json() : {};
    if (config.dataset_url) {
      $("#dataset-link").href = config.dataset_url;
      $("#dataset-link").hidden = false;
    }
    $("#map-count").textContent = state.maps.length.toLocaleString("zh-CN");
    $("#total-size").textContent = formatBytes(state.maps.reduce((sum, map) => sum + Number(map.size || 0), 0));
    const generatedAt = payload.generated_at || (payload.generated_at_unix ? payload.generated_at_unix * 1000 : null);
    $("#updated-at").textContent = generatedAt ? new Date(generatedAt).toLocaleDateString("zh-CN") : "未知";
    buildCategories();
    applyFilters();
  } catch (error) {
    $("#notice").textContent = `地图索引载入失败：${error.message}`;
  }
}

$("#search-form").addEventListener("submit", (event) => event.preventDefault());
$("#search-input").addEventListener("input", (event) => { state.query = event.target.value; applyFilters(); });
$("#sort-select").addEventListener("change", (event) => { state.sort = event.target.value; applyFilters(); });
$("#load-more").addEventListener("click", () => { state.shown += PAGE_SIZE; render(); });
start();

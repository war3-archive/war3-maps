// One controller for both views.
//
// Browsing a category and searching the archive differ only in which maps are in
// the result set and in what order — the cards, the paging and the shard loading
// are identical. Keeping them in one place is what lets the search box live on
// category pages too, instead of sending people back to the home page.
//
// A result set is a list of {collection, offset} locators; card data is read out
// of the category shards those locators point into (see shards.js).

import {
  BASE,
  PAGE_SIZE,
  attachDetail,
  buildCard,
  collator,
  collections,
  fetchJSON,
  normalize,
} from "./map-browser.js";
import { resolve } from "./shards.js";

// Search index tuple, as written by scripts/generate-site.mjs.
const NAME = 0;
const AUTHOR = 1;
const FILENAME = 2;
const COLLECTION = 3;
const OFFSET = 4;
const SIZE = 5;
const PLAYERS = 6;

const results = document.querySelector("#results");
const notice = document.querySelector("#notice");
const more = document.querySelector("#load-more");
const input = document.querySelector("#search-input");
const form = document.querySelector("#search-form");
const sortSelect = document.querySelector("#sort-select");
const scopeToggle = document.querySelector("#search-scope");

// Set on category pages; null on the home page.
const homeCollection = results.dataset.collection
  ? collections.findIndex((entry) => entry.name === results.dataset.collection)
  : null;
const homeTotal = Number(results.dataset.total || 0);

const state = {
  index: null,
  haystack: null,
  query: "",
  sort: "relevance",
  source: null,
  rendered: 0,
  start: 0,
  token: 0,
};

/** A browse source enumerates one category in its stored (name-sorted) order. */
function browseSource(collection, total, start = 0) {
  return {
    kind: "browse",
    total: total - start,
    at: (index) => ({ collection, offset: start + index }),
  };
}

function searchSource(matches) {
  return {
    kind: "search",
    total: matches.length,
    at: (index) => ({ collection: matches[index][COLLECTION], offset: matches[index][OFFSET] }),
  };
}

async function ensureIndex() {
  if (state.index) return;
  const payload = await fetchJSON(`${BASE}data/search-index.json`);
  state.index = payload.maps;
  // Normalizing once here is what keeps each keystroke a plain substring test.
  state.haystack = payload.maps.map((entry) =>
    normalize(
      `${entry[NAME]} ${entry[AUTHOR]} ${entry[FILENAME] || ""} ${
        collections[entry[COLLECTION]]?.name || ""
      }`,
    ),
  );
}

/**
 * Where the query matched, most specific first. Alphabetical order buries the
 * obvious answer: "塔防" matches 755 maps, and the one actually called 塔防 has
 * no reason to sit in the middle of them.
 *
 * Scored only over the hits, so the cost tracks the result count rather than
 * the size of the archive.
 */
function relevance(entry, query, terms) {
  const name = normalize(entry[NAME]);
  if (name === query) return 6000;
  if (name.startsWith(query)) return 5000 - Math.min(name.length, 400);
  if (name.includes(query)) return 4000 - Math.min(name.length, 400);
  if (terms.length > 1 && terms.every((term) => name.includes(term))) {
    return 3000 - Math.min(name.length, 400);
  }
  const author = normalize(entry[AUTHOR]);
  if (author === query) return 2500;
  if (terms.every((term) => author.includes(term))) return 2000;
  const filename = normalize(entry[FILENAME] || "");
  if (terms.every((term) => filename.includes(term))) return 1000;
  // Everything left matched by category name, or by different terms hitting
  // different fields.
  return 0;
}

function sortMatches(matches, query, terms) {
  if (state.sort === "size-desc") {
    matches.sort((a, b) => Number(b[SIZE] || 0) - Number(a[SIZE] || 0));
  } else if (state.sort === "players-desc") {
    matches.sort((a, b) => Number(b[PLAYERS] || 0) - Number(a[PLAYERS] || 0));
  } else if (state.sort === "relevance" && terms.length > 0) {
    const scores = new Map();
    for (const entry of matches) scores.set(entry, relevance(entry, query, terms));
    matches.sort(
      (a, b) => scores.get(b) - scores.get(a) || collator.compare(a[NAME] || "", b[NAME] || ""),
    );
  } else {
    matches.sort((a, b) => collator.compare(a[NAME] || "", b[NAME] || ""));
  }
  return matches;
}

function setNotice(text) {
  notice.hidden = !text;
  if (text) notice.textContent = text;
}

function updateMore() {
  const total = state.source?.total ?? 0;
  more.hidden = state.rendered >= total;
  more.textContent = `显示更多（${state.rendered} / ${total}）`;
}

async function renderMore(count = PAGE_SIZE) {
  const source = state.source;
  if (!source) return 0;
  const token = state.token;
  const end = Math.min(state.rendered + count, source.total);
  const locators = [];
  for (let index = state.rendered; index < end; index += 1) locators.push(source.at(index));
  if (locators.length === 0) {
    updateMore();
    return 0;
  }

  const tuples = await resolve(locators);
  if (token !== state.token) return 0; // a newer query superseded this render

  const fragment = document.createDocumentFragment();
  for (let index = 0; index < tuples.length; index += 1) {
    const tuple = tuples[index];
    if (!tuple) continue;
    fragment.append(buildCard(tuple, collections[locators[index].collection]?.name));
  }
  results.append(fragment);
  state.rendered = end;
  updateMore();
  return locators.length;
}

/** Replace the result set and repaint from the top. */
async function setSource(source, emptyMessage) {
  state.token += 1;
  state.source = source;
  state.rendered = 0;
  results.replaceChildren();
  if (!source || source.total === 0) {
    updateMore();
    setNotice(emptyMessage);
    return;
  }
  setNotice("");
  await renderMore();
}

async function applyQuery() {
  const query = normalize(state.query).trim();
  const terms = query.split(/\s+/).filter(Boolean);
  const browsing = terms.length === 0;

  if (browsing && homeCollection === null) {
    await setSource(null, "输入关键词开始搜索，或选择一个分类浏览。");
    return;
  }
  // Shards are already stored in name order, so plain browsing needs no index.
  if (browsing && (state.sort === "name" || state.sort === "relevance")) {
    await setSource(browseSource(homeCollection, homeTotal, state.start), "这个分类下没有地图。");
    return;
  }

  setNotice("正在检索…");
  const token = ++state.token;
  await ensureIndex();
  if (token !== state.token) return;

  // Re-sorting a category, like searching, needs every candidate up front.
  const withinCategory = browsing || (homeCollection !== null && scopeToggle?.checked);
  const { index, haystack } = state;
  const matches = [];
  for (let position = 0; position < index.length; position += 1) {
    if (withinCategory && index[position][COLLECTION] !== homeCollection) continue;
    if (!browsing) {
      const text = haystack[position];
      let hit = true;
      for (const term of terms) {
        if (!text.includes(term)) {
          hit = false;
          break;
        }
      }
      if (!hit) continue;
    }
    matches.push(index[position]);
  }
  await setSource(
    searchSource(sortMatches(matches, query, terms)),
    browsing
      ? "这个分类下没有地图。"
      : withinCategory
        ? "这个分类里没有匹配的地图，取消“仅本分类”试试。"
        : "没有匹配的地图，试试减少关键词。",
  );
}

function runQuery() {
  applyQuery().catch((error) => {
    setNotice(`载入失败：${error.message}`);
  });
}

let debounce;
input?.addEventListener("input", (event) => {
  state.query = event.target.value;
  clearTimeout(debounce);
  debounce = setTimeout(runQuery, 150);
});
form?.addEventListener("submit", (event) => {
  event.preventDefault();
  clearTimeout(debounce);
  runQuery();
});
scopeToggle?.addEventListener("change", runQuery);
sortSelect?.addEventListener("change", (event) => {
  state.sort = event.target.value;
  runQuery();
});
more.addEventListener("click", () => {
  more.disabled = true;
  renderMore().finally(() => {
    more.disabled = false;
  });
});
attachDetail(results);

// Deep link from a search result: start the category at that map.
if (homeCollection !== null) {
  const requested = Number(new URLSearchParams(location.search).get("i"));
  if (Number.isFinite(requested) && requested > 0) {
    state.start = Math.min(requested, Math.max(homeTotal - 1, 0));
  }
}

applyQuery()
  .then(() => {
    if (state.start > 0) {
      const first = results.firstElementChild;
      if (first) {
        first.classList.add("map-card--highlight");
        const back = document.createElement("a");
        back.className = "load-more back-to-start";
        back.href = `${BASE}categories/${collections[homeCollection].slug}/`;
        back.textContent = `← 从头浏览「${collections[homeCollection].name}」全部 ${homeTotal} 张`;
        results.before(back);
      }
    }
  })
  .catch((error) => setNotice(`载入失败：${error.message}`));

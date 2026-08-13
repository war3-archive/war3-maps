// Category view: the page ships an empty grid and pulls 500-record shards on
// demand, so opening 未分类地图 costs ~3 KB of HTML instead of 3.8 MB.
//
// Rendering is driven by an absolute cursor into the category, which lets a
// search deep link (?i=<offset>) start at the shard holding that map instead of
// rendering everything ahead of it.

import { BASE, PAGE_SIZE, SHARD_SIZE, attachCopyHash, buildCard, fetchJSON } from "./map-browser.js";

const results = document.querySelector("#results");
const notice = document.querySelector("#notice");
const more = document.querySelector("#load-more");

const slug = results.dataset.slug;
const collectionName = results.dataset.collection;
const total = Number(results.dataset.total || 0);

const shards = new Map();
const requested = Number(new URLSearchParams(location.search).get("i"));
const start = Number.isFinite(requested) ? Math.min(Math.max(requested, 0), Math.max(total - 1, 0)) : 0;
let cursor = start;

async function shardAt(index) {
  const page = Math.floor(index / SHARD_SIZE);
  if (!shards.has(page)) {
    shards.set(
      page,
      fetchJSON(`${BASE}data/categories/${slug}/${page}.json`).then((shard) => shard.maps),
    );
  }
  return shards.get(page);
}

function updateMore() {
  more.hidden = cursor >= total;
  more.textContent = `显示更多（${cursor - start} / ${total - start}）`;
}

async function renderMore(count = PAGE_SIZE) {
  const end = Math.min(cursor + count, total);
  const fragment = document.createDocumentFragment();
  while (cursor < end) {
    const maps = await shardAt(cursor);
    const offset = cursor % SHARD_SIZE;
    const take = Math.min(maps.length - offset, end - cursor);
    if (take <= 0) break;
    for (let index = offset; index < offset + take; index += 1) {
      fragment.append(buildCard(maps[index], collectionName));
    }
    cursor += take;
  }
  results.append(fragment);
  notice.hidden = cursor > start;
  updateMore();
}

/** Deep links land on the requested map; offer a way back to the top of the list. */
function markEntryPoint() {
  if (start === 0) return;
  const first = results.firstElementChild;
  if (first) first.classList.add("map-card--highlight");
  const back = document.createElement("a");
  back.className = "load-more back-to-start";
  back.href = `${BASE}categories/${slug}/`;
  back.textContent = `← 从头浏览「${collectionName}」全部 ${total} 张`;
  results.before(back);
}

attachCopyHash(results);
more.addEventListener("click", () => {
  more.disabled = true;
  renderMore().finally(() => {
    more.disabled = false;
  });
});

renderMore()
  .then(markEntryPoint)
  .catch((error) => {
    notice.hidden = false;
    notice.textContent = `地图列表载入失败：${error.message}`;
  });

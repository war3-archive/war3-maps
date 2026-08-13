// Category shards are the single source of card data.
//
// Search results are located by (collection, offset) rather than carrying their
// own copy of every display field, so the search index stays an index — it holds
// only what is needed to match, sort and locate — and the shards it points into
// are the same files category browsing already downloads and the browser already
// caches. A page of 48 results touches 2-9 of the 33 shards in practice.

import { BASE, SHARD_SIZE, collections, fetchJSON } from "./map-browser.js";

const cache = new Map();

const shardKey = (collectionIndex, page) => `${collectionIndex}/${page}`;

/** Fetch (once) one shard of one collection. */
export function shardFor(collectionIndex, page) {
  const key = shardKey(collectionIndex, page);
  if (!cache.has(key)) {
    const slug = collections[collectionIndex]?.slug;
    cache.set(
      key,
      fetchJSON(`${BASE}data/categories/${slug}/${page}.json`).then((shard) => shard.maps),
    );
  }
  return cache.get(key);
}

/**
 * Resolve {collection, offset} locators to card tuples, fetching each distinct
 * shard at most once and all of them in parallel.
 *
 * @returns tuples in the same order as `locators`; null where a shard had no
 *   record at that offset.
 */
export async function resolve(locators) {
  const pages = new Map();
  for (const { collection, offset } of locators) {
    const page = Math.floor(offset / SHARD_SIZE);
    pages.set(shardKey(collection, page), [collection, page]);
  }
  const loaded = new Map();
  await Promise.all(
    [...pages].map(async ([key, [collection, page]]) => {
      loaded.set(key, await shardFor(collection, page));
    }),
  );

  return locators.map(({ collection, offset }) => {
    const page = Math.floor(offset / SHARD_SIZE);
    const maps = loaded.get(shardKey(collection, page));
    return maps?.[offset % SHARD_SIZE] ?? null;
  });
}

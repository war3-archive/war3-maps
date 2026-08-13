---
pretty_name: Warcraft III Community Map Archive
language:
  - zh
  - en
tags:
  - games
  - archive
  - warcraft-iii
---

# Warcraft III Community Map Archive

This public dataset preserves community-created Warcraft III maps and campaigns
for interoperability testing, search, research, and long-term access. Files are
deduplicated by SHA-256. Titles and other metadata are extracted with
[`war3-manager`](https://github.com/war3-archive/war3-manager) where the format permits.

- Search and download individual maps: https://war3-archive.github.io/war3-maps/
- Source and issue tracker: https://github.com/war3-archive/war3-maps

## Layout

- `objects/<sha256-prefix>/<sha256>.<ext>`: immutable map or campaign objects
- `covers/<sha256-prefix>/<sha256>.webp`: minimap / preview thumbnail, when one
  could be extracted; the same SHA-256 as the object it belongs to
- `catalog/maps.json`: versioned website catalog
- `catalog/maps.jsonl`: one record per line for downstream processing
- `catalog/failures.json`: inputs that could not be extracted or parsed

Catalog records point at both files by path and by resolve URL (`dataset_path` /
`download_url`, `cover_path` / `cover_url`), so the catalog stays small enough to
fetch directly from a browser.

## Acknowledgements

Special thanks to Bilibili creator
[关先生丶的游戏实况](https://space.bilibili.com/2534568), whose collection and
sharing work is the source of a substantial part of the classic maps here.

## Rights

The original maps remain works of their respective authors. Inclusion in this
archive does not claim ownership or grant rights beyond those provided by each
author. Source provenance is retained in the catalog. For attribution fixes,
copyright concerns, or takedown requests, open an issue at
https://github.com/war3-archive/war3-maps/issues with the SHA-256 and supporting
details. Confirmed requests will be handled by removing the public object and
recording a tombstone in the catalog.

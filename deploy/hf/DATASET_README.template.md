---
pretty_name: Warcraft III Community Map Archive
license: other
license_name: community-authored-maps
license_link: https://huggingface.co/datasets/magicwenli/war3-maps#rights
language:
  - zh
  - en
tags:
  - games
  - archive
  - warcraft-iii
  - warcraft
  - w3x
  - mpq
  - digital-preservation
size_categories:
  - 10K<n<100K
configs:
  - config_name: maps
    data_files:
      - split: train
        path: catalog/maps.jsonl
dataset_info:
  config_name: maps
  features:
    - name: sha256
      dtype: string
    - name: name
      dtype: string
    - name: author
      dtype: string
    - name: description
      dtype: string
    - name: category
      dtype: string
    - name: collection
      dtype: string
    - name: collections
      sequence: string
    - name: filename
      dtype: string
    - name: extension
      dtype: string
    - name: format
      dtype: string
    - name: content_type
      dtype: string
    - name: size
      dtype: int64
    - name: recommended_players
      dtype: string
    - name: max_players
      dtype: int64
    - name: player_count
      dtype: int64
    - name: playable_width
      dtype: int64
    - name: playable_height
      dtype: int64
    - name: tileset
      dtype: int64
    - name: format_version
      dtype: int64
    - name: editor_version
      dtype: int64
    - name: build_version
      sequence: int64
    - name: name_source
      dtype: string
    - name: parse_status
      dtype: string
    - name: parse_error
      dtype: string
    - name: modification
      struct:
        - name: tool
          dtype: string
        - name: label
          dtype: string
        - name: variant
          dtype: string
        - name: activation
          sequence: string
        - name: evidence
          sequence: string
        - name: reference
          dtype: string
    - name: cover_status
      dtype: string
    - name: cover_source
      dtype: string
    - name: cover_path
      dtype: string
    - name: cover_url
      dtype: string
    - name: dataset_path
      dtype: string
    - name: download_url
      dtype: string
    - name: source_paths
      sequence: string
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

## Dataset viewer

The preview and `load_dataset` both read **only** `catalog/maps.jsonl` — one row
per archived object. The map files themselves are binary MPQ archives and are
not loadable as tabular data; fetch them from `download_url` (or `dataset_path`
via `hf_hub_download`) once you have selected rows from the catalog.

```python
from datasets import load_dataset

catalog = load_dataset("magicwenli/war3-maps", split="train")
melee = catalog.filter(lambda row: row["content_type"] == "map" and row["max_players"] == 2)
```

The record schema is pinned in this card's metadata, so a new catalog field is
invisible to the viewer until it is added there as well.

### Catalog fields

| Field | Meaning |
|-------|---------|
| `sha256`, `size`, `filename`, `extension`, `format`, `content_type` | object identity; `content_type` is `map` or `campaign` |
| `name`, `author`, `description`, `category`, `collection`, `collections` | presentation metadata; `name_source` records where the title came from (`w3i`, filename, …) |
| `recommended_players`, `max_players`, `player_count`, `playable_width`, `playable_height`, `tileset` | gameplay metadata parsed from `war3map.w3i` |
| `format_version`, `editor_version`, `build_version` | map format and authoring-tool versions; `build_version` is a 4-part list when known |
| `parse_status`, `parse_error` | `ok`, `carved` (salvaged from raw sectors), `metadata_error`, or `metadata_unavailable` (campaign packs are not parsed for metadata) |
| `modification` | third-party script modification detected in the map, when any: tool, label, variant, activation strings and evidence |
| `cover_status`, `cover_source`, `cover_path`, `cover_url` | thumbnail availability and origin (`preview` or minimap) |
| `dataset_path`, `download_url`, `source_paths` | where the object lives here, and the original paths it was ingested from |

## Acknowledgements

Special thanks to Bilibili creator
[关先生丶的游戏实况](https://space.bilibili.com/2534568), whose collection and
sharing work is the source of a substantial part of the classic maps here.

## Rights

The original maps remain works of their respective authors. Inclusion in this
archive does not claim ownership or grant rights beyond those provided by each
author. The `other` license tag on this dataset refers to this section, not to a
single license covering the contents. Source provenance is retained in the
catalog. For attribution fixes, copyright concerns, or takedown requests, open an
issue at https://github.com/war3-archive/war3-maps/issues with the SHA-256 and
supporting details. Confirmed requests will be handled by removing the public
object and recording a tombstone in the catalog.

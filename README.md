# war3parser

[![Crates.io Version](https://img.shields.io/crates/v/war3parser)](https://crates.io/crates/war3parser)
[![docs.rs](https://img.shields.io/docsrs/war3parser)](https://docs.rs/war3parser)
[![NPM Version](https://img.shields.io/npm/v/%40wesleyel%2Fwar3parser)](https://www.npmjs.com/package/@wesleyel/war3parser)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/wesleyel/war3parser/build.yml)](https://github.com/wesleyel/war3parser/actions/workflows/build.yml)
[![GitHub Release](https://img.shields.io/github/v/release/wesleyel/war3parser)](https://github.com/wesleyel/war3parser/releases)

`war3parser` is a library for parsing and extracting Warcraft III map files. It extracts data from MPQ archives and parses common map formats across classic and Reforged versions — the `w3i` parser covers the full format ladder **v8 → v33** (RoC betas through WC3 2.0).

## Workspace layout

```text
crates/
  core/   # war3parser        — pure parsing + shared model (no wasm-bindgen by default)
  cli/    # war3parser-cli    — thin CLI over core
  wasm/   # war3parser-wasm   — thin wasm-bindgen glue over core::model::MapSnapshot
```

| Crate | Depends on | Notes |
|-------|------------|-------|
| `war3parser` (core) | — | pure Rust + optional `serde` (default); **no** wasm-bindgen/tsify |
| `war3parser-cli` | core + `serde` | never pulls wasm-bindgen |
| `war3parser-wasm` | core + `serde-wasm-bindgen` | thin `parse_map` / `version`; hand-maintained `war3parser.d.ts` |

Core module layout:

```text
crates/core/src/
  archive.rs   # War3MapW3x — HM3W header + embedded MPQ access
  formats/     # per-file parsers: w3i (v8→v33), wts, imp, mmp
  model/       # portable API types: MapSnapshot, War3MapMetadata, images
  reader.rs    # bounds-checked little-endian ByteReader
  error.rs     # crate-wide Error
```

Shared API types (`MapSnapshot`, `War3ImageData`, `ImportEntry`, `StringTableEntry`, `War3MapHeader`, …) live in `war3parser::model` so CLI and WASM do not redefine DTOs.

## Features

- Extract files from MPQ archives (by known name)
- Parse **w3i** map info across versions **8 → 33** (RoC betas, ROC, TFT, 1.31+, Reforged, WC3 2.0)
- Parse **wts** string tables (comment lines, `\n` / `\r\n`, BOM)
- Parse **imp** imports, minimap/preview **BLP/TGA** images
- Handle protected / headerless maps (no `HM3W`, truncated optional w3i sections, missing listfile,
  inflated MPQ table counts, hash entries that wrap around the table)
- Detect known third-party script modifications (`war3parser::modscan`)
- WASM bindings + browser playground

### MPQ reader

MPQ reading comes from [`war3-mpq`](https://crates.io/crates/war3-mpq), a fork of
`mpq` 0.8.1. Upstream trusts values that come straight out of the archive —
block-table indices, per-sector offsets, table counts — which map protectors
deliberately falsify, and it stops probing the hash table at the end instead of
wrapping around, so some files are unreachable by name. Over a 10365-map archive
the fixes took readable maps from 9218 to 9746.

It is pulled in under the name `mpq`, so the source reads the same:

```toml
mpq = { package = "war3-mpq", version = "0.9" }
```

A `[patch.crates-io]` would have been invisible to anyone installing this crate
from the registry — Cargo only honours `[patch]` from the final workspace root.

### Modification detection

`modscan` recognises injected cheat scripts from literals the injector cannot
remove, and reports how the injected menu is triggered in game. It needs only
the map at hand — no unmodified copy to diff against. `parse_map` exposes it as
`modification`; `dump-metadata` writes `modification.json`. A `None` result
means "no known signature matched", not "clean": a protected map whose script
cannot be read looks the same.

## Usage

### as a library

```bash
cargo add war3parser
```

```rust
use war3parser::prelude::War3MapMetadata;

let buffer = std::fs::read("path/to/map.w3x").unwrap();
let mut metadata = War3MapMetadata::parse(&buffer).unwrap();
metadata.resolve_trigger_strings();

// Portable snapshot shared with the WASM API
let snapshot = metadata.snapshot().unwrap();
println!("{:?}", snapshot.map_info.as_ref().map(|i| &i.name));

metadata.save("out").unwrap();
// or: War3MapMetadata::parse_snapshot(&buffer)
```

### as a CLI

```bash
cargo install war3parser-cli
```

```plaintext
$ war3parser-cli help
A extractor and parser for Warcraft 3 map files

Usage: war3parser-cli <COMMAND>

Commands:
  dump-metadata   Dump metadata from a map file [aliases: d]
  extract-file    Extract a file from a MPQ archive and save it [aliases: x]
  extract-images  Extract images with *.tga and *.blp extensions [aliases: i]
  convert-image   Convert a *tga/blp file to png [aliases: c]
  list-files      List files in a MPQ archive [aliases: l]
  help            Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### as WASM

```bash
npm install @wesleyel/war3parser
```

```javascript
import init, { parse_map, parse_map_result, list_files, extract_file, detect_modification, version }
  from "@wesleyel/war3parser";

await init();
const meta = parse_map(new Uint8Array(buffer));
console.log(version(), meta?.map_info?.name, meta?.strings?.length);

// Why a map failed, instead of a bare `undefined`
const result = parse_map_result(new Uint8Array(buffer));
if (!result.ok) console.warn(result.error);

// Reach into the archive directly
const script = extract_file(new Uint8Array(buffer), "war3map.j");
const files = list_files(new Uint8Array(buffer));
const injected = detect_modification(new Uint8Array(buffer));
```

`parse_map` returns:

- `header` — HM3W presence/name/max players
- `map_info` — full w3i (TRIGSTR-resolved when `.wts` is present)
- `images` — minimap/preview as PNG data URLs
- `imports` — `war3map.imp` entries
- `strings` — sorted WTS entries
- `files` — `(listfile)` paths when available
- `parse_ms` — parse duration

`get_map_info` remains as a compatible alias of `parse_map`.

### Web playground

Live: **https://wesleyel.github.io/war3parser/**

Local demo (builds WASM first):

```bash
just serve-playground
# → http://localhost:5173/
```

Drop any `.w3x` / `.w3m`. Parsing is 100% in-browser; nothing is uploaded.

Static / GitHub Pages build:

```bash
just build-playground   # relative base, output: playground/dist-site/
just build-pages        # base=/war3parser/ for GitHub Pages
```

The playground is Vite + React with a Real World Materials UI. CI deploys it to GitHub Pages on pushes to `main` (see `.github/workflows/pages.yml`). Enable **Settings → Pages → Source: GitHub Actions** once.

## w3i version support

The version ladder follows [War3Net](https://github.com/Drake53/War3Net), the most
complete open reference:

| Version | Era | Notes |
|--------:|-----|-------|
| 8–15 | RoC beta | Legacy layouts (no save count pre-18, no subtitles pre-15) |
| 18 | ROC | Campaign background, loading screen index |
| 23–24 | early TFT | Fog, sound environment, game data set; random item tables (24) |
| 25 | TFT | Global weather |
| 26–27 | TFT patches | Trailing marker int (26–27), game build version (27) |
| 28 | 1.31 | Script language (JASS/Lua) |
| 31 | Reforged | Graphics modes, game data version, enemy priorities |
| 32–33 | WC3 2.0 | Camera zoom limits (32: default+max, 33: +min) |
| * | unknown | Future/gap versions parse with the nearest known layout |
| * | protected | `0xFF` optional-section skip after forces; tolerant truncation |

## Contributing

Contributions are welcome! Please submit a Pull Request or report an Issue.

## License

`war3parser` is licensed under the MIT License. See the LICENSE file for details.

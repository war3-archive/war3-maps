# war3parser

[![Crates.io Version](https://img.shields.io/crates/v/war3parser)](https://crates.io/crates/war3parser)
[![docs.rs](https://img.shields.io/docsrs/war3parser)](https://docs.rs/war3parser)
[![NPM Version](https://img.shields.io/npm/v/%40wesleyel%2Fwar3parser)](https://www.npmjs.com/package/@wesleyel/war3parser)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/wesleyel/war3parser/build.yml)](https://github.com/wesleyel/war3parser/actions/workflows/build.yml)
[![GitHub Release](https://img.shields.io/github/v/release/wesleyel/war3parser)](https://github.com/wesleyel/war3parser/releases)

`war3parser` is a library for parsing and extracting Warcraft III map files. It extracts data from MPQ archives and parses common map formats across classic and Reforged versions.

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

Shared API types (`MapSnapshot`, `War3ImageData`, `ImportEntry`, `StringTableEntry`, `War3MapHeader`, …) live in `war3parser::model` so CLI and WASM do not redefine DTOs.

## Features

- Extract files from MPQ archives (by known name)
- Parse **w3i** map info across versions **18 → 33** (ROC, TFT, 1.31+, Reforged, WC3 2.0)
- Parse **wts** string tables (comment lines, `\n` / `\r\n`, BOM)
- Parse **imp** imports, minimap/preview **BLP/TGA** images
- Handle protected / headerless maps (no `HM3W`, truncated optional w3i sections, missing listfile)
- WASM bindings + browser playground

## Usage

### as a library

```bash
cargo add war3parser
```

```rust
use war3parser::prelude::War3MapMetadata;

let buffer = std::fs::read("path/to/map.w3x").unwrap();
let mut metadata = War3MapMetadata::from(&buffer).unwrap();
metadata.update_string_table().ok();

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
import init, { parse_map, version } from "@wesleyel/war3parser";

await init();
const meta = parse_map(new Uint8Array(buffer));
console.log(version(), meta?.map_info?.name, meta?.strings?.length);
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

Local demo (builds WASM first):

```bash
just serve-playground
# → http://localhost:5173/
```

Drop any `.w3x` / `.w3m`. Parsing is 100% in-browser; nothing is uploaded.

Static build:

```bash
just build-playground
# output: playground/dist-site/
```

## w3i version support

| Version | Era | Notes |
|--------:|-----|-------|
| 18 | ROC | Base layout |
| 25 | TFT | Loading models, fog, random item tables |
| 28 | 1.31 | Build version + script language |
| 31 | Reforged | Graphics modes, game data version, enemy priorities |
| 32–33 | WC3 2.0 | Camera zoom defaults |
| * | protected | `0xFF` optional-section skip after forces |

## Contributing

Contributions are welcome! Please submit a Pull Request or report an Issue.

## License

`war3parser` is licensed under the MIT License. See the LICENSE file for details.

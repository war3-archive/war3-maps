# Local improvement notes (0.3.x)

## Workspace structure
- Renamed `crates/lib` → `crates/core` (package name still `war3parser`)
- Core split: `parser` (format readers) + `model` (shared portable types)
- Features: `default = ["serde"]`; `typescript` only for wasm; `wasm` deprecated alias
- CLI depends on core+serde only (no wasm-bindgen)
- WASM is thin glue over `MapSnapshot`; removed duplicated DTOs
- Shared types: `War3MapHeader`, `War3ImageData`, `ImportEntry`, `StringTableEntry`, `MapSnapshot`

## Parser
- Fix w3i `0xFF` optional-section skip (DotA classic)
- Fix RandomUnitTable row count
- Gate random item tables to version > 24
- fog_density as f32
- Camera zoom fields for w3i v32/v33
- Player enemy priority fields (v31+)
- Robust WTS parser (comments, LF/CRLF, BOM)
- IMP standard-path flags 0/1/8
- listfile splits on `\n`/`\r`
- War3MapHeader + files on metadata

## WASM
- `parse_map`, `version`, richer metadata (imports/strings/files/header/parse_ms)
- `get_map_info` alias retained
- wasm-opt disabled (bulk-memory incompatibility)

## Playground
- Vite + vanilla TS under `playground/`
- `just serve-playground` / `just build-playground`

# Manager tooling. Parsing lives upstream in war3parser.

build:
    cargo build -p war3-manager-cli

# Optimized build used for bulk cataloging (panic = unwind, see Cargo.toml).
build-catalog:
    cargo build --profile catalog -p war3-manager-cli

lint:
    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

# Refresh an existing dataset after an upstream parser fix.
rescan dataset:
    cargo run --profile catalog -p war3-manager-cli -- rescan {{dataset}} -o rescan.jsonl
    python3 deploy/apply_rescan.py {{dataset}} rescan.jsonl
    python3 deploy/export_covers.py {{dataset}}

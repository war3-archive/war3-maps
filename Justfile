# Workspace tooling: the MPQ reader, the parser, the manager CLI and the site
# all live here now, sharing one version.

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

# Bump every crate together and publish bottom-up. Needs `cargo install cargo-release`.
release level:
    cargo release {{level}} --workspace

# Refresh an existing dataset after an upstream parser fix.
rescan dataset:
    cargo run --profile catalog -p war3-manager-cli -- rescan {{dataset}} -o rescan.jsonl
    python3 deploy/apply_rescan.py {{dataset}} rescan.jsonl
    python3 deploy/export_covers.py {{dataset}}

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
    uv run --project deploy ruff format deploy/src
    uv run --project deploy ruff check deploy/src

test:
    cargo test --workspace --all-targets

# Bump every crate together and publish bottom-up. Needs `cargo install cargo-release`.
#
# Publishing itself happens in CI — the crates.io token and npm OIDC identity
# live there — so this only moves the version and pushes the tag that triggers
# it. Add --execute to do it for real; without it cargo-release prints the plan.
release level:
    cargo release {{level}} --workspace

# Refresh an existing dataset after an upstream parser fix.
rescan dataset:
    cargo run --profile catalog -p war3-manager-cli -- rescan {{dataset}} -o rescan.jsonl
    uv run --project deploy war3-deploy apply-rescan {{dataset}} rescan.jsonl
    uv run --project deploy war3-deploy export-covers {{dataset}}

# The deploy tooling is a uv project; `uv run` syncs it on demand.
deploy *args:
    uv run --project deploy war3-deploy {{args}}

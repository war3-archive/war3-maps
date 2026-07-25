clean-wasm:
    rm -rf dist
    mkdir -p dist

build-wasm: clean-wasm
    wasm-pack build {{justfile_directory()}}/crates/wasm --out-name war3parser --target web --out-dir ../../dist --scope wesleyel

build-core:
    cargo build -p war3parser

build-cli:
    cargo build -p war3parser-cli

build: build-wasm build-core build-cli

lint:
    cargo fmt --all
    cargo clippy --workspace --all-targets --features serde -- -D warnings
    cargo clippy -p war3parser-wasm --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets --features serde
    cargo test -p war3parser --features typescript --all-targets

serve-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run dev

build-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run build

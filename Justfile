clean-wasm:
    rm -rf dist
    mkdir -p dist

build-wasm: clean-wasm
    wasm-pack build {{justfile_directory()}}/crates/wasm --out-name war3parser --target web --out-dir ../../dist --scope wesleyel

build-lib:
    cd {{justfile_directory()}}/crates/lib && cargo build

build-cli:
    cd {{justfile_directory()}}/crates/cli && cargo build

build: build-wasm build-lib build-cli

lint:
    cd {{justfile_directory()}} && cargo fmt --all
    cd {{justfile_directory()}} && cargo clippy --all-targets --all-features -- -D warnings

test:
    cd {{justfile_directory()}} && cargo test --all-targets --all-features

serve-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run dev

build-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run build

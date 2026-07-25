clean-wasm:
    rm -rf dist
    mkdir -p dist

# Build WASM package and overlay the hand-maintained TypeScript definitions.
build-wasm: clean-wasm
    wasm-pack build {{justfile_directory()}}/crates/wasm --out-name war3parser --target web --out-dir ../../dist --scope wesleyel
    cp {{justfile_directory()}}/crates/wasm/war3parser.d.ts {{justfile_directory()}}/dist/war3parser.d.ts
    # npm package name: @wesleyel/war3parser (wasm-pack uses crate name by default)
    node -e "const fs=require('fs');const p='dist/package.json';const j=JSON.parse(fs.readFileSync(p,'utf8'));j.name='@wesleyel/war3parser';j.types='war3parser.d.ts';fs.writeFileSync(p, JSON.stringify(j,null,2)+'\n')"

build-core:
    cargo build -p war3parser

build-cli:
    cargo build -p war3parser-cli

build: build-wasm build-core build-cli

lint:
    cargo fmt --all
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

serve-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run dev

build-playground: build-wasm
    cd {{justfile_directory()}}/playground && npm install && npm run build

# Production build for GitHub Pages (base = /war3parser/)
build-pages: build-wasm
    cd {{justfile_directory()}}/playground && npm install && GITHUB_PAGES=1 npm run build

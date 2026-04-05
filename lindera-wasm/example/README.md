# Lindera WASM Example

An interactive web application for morphological analysis using Lindera compiled to WebAssembly.

## Prerequisites

- Rust toolchain
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (`cargo install wasm-pack`)
- Node.js (v18+)

All commands below should be run from the repository root (`lindera/`) unless otherwise noted.

## Build

### 1. Build the WASM package

```bash
cd lindera-wasm
wasm-pack build --release --target=web
cp js/opfs.js pkg/
cp js/opfs.d.ts pkg/
```

This generates the `pkg/` directory containing `.wasm` files and JavaScript glue code.

### 2. Install npm dependencies and start

```bash
cd example
npm install
```

## Run

### Development server

From `lindera-wasm/example/`:

```bash
npm start
```

Then open <http://localhost:8080> in your browser.

The dev server is configured with:

- CORS headers for OPFS (Origin Private File System) access
- Proxy for GitHub Releases to avoid CORS issues during dictionary downloads

### Production build

From `lindera-wasm/example/`:

```bash
npm run build
```

Output files are generated in the `dist/` directory.

## How it works

The web application downloads dictionary files (e.g., IPADIC) from GitHub Releases and caches them in the browser's OPFS. You can manage dictionaries (download/delete) from the UI and run morphological analysis interactively.

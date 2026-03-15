# Lindera WASM

Lindera WASM provides WebAssembly bindings for Lindera's morphological analysis engine, built with [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/). It enables Japanese, Korean, and Chinese text tokenization directly in web browsers, Node.js, and bundler environments.

## Distribution Formats

Lindera WASM supports multiple distribution formats via [wasm-pack](https://rustwasm.github.io/wasm-pack/):

| Target | Use Case | Module System |
| --- | --- | --- |
| `web` | Browser ESM | ES Modules |
| `nodejs` | Node.js | CommonJS |
| `bundler` | Webpack, Vite, Rollup | ES Modules (bundler-resolved) |

## Dictionary Packages

Each package embeds a specific dictionary for offline use:

| Feature Flag | Dictionary | Language |
| --- | --- | --- |
| (none) | No embedded dictionary | -- |
| `embed-ipadic` | IPADIC | Japanese |
| `embed-unidic` | UniDic | Japanese |
| `embed-ko-dic` | ko-dic | Korean |
| `embed-cc-cedict` | CC-CEDICT | Chinese |
| `embed-jieba` | Jieba | Chinese |
| `embed-cjk` | IPADIC + ko-dic + Jieba | CJK |

## Sections

- [Installation](./lindera-wasm/installation.md) -- Building and installing lindera-wasm packages
- [Quick Start](./lindera-wasm/quickstart.md) -- Minimal working example
- [Tokenizer API](./lindera-wasm/tokenizer_api.md) -- Full API reference for JavaScript/TypeScript
- [Dictionary Management](./lindera-wasm/dictionary_management.md) -- Loading and building dictionaries
- [Browser Usage](./lindera-wasm/browser_usage.md) -- Integration with web applications
- [Node.js Usage](./lindera-wasm/nodejs_usage.md) -- Server-side usage with Node.js

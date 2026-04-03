# Installation

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (v0.10+)

## Building with wasm-pack

Build the WASM package for your target environment. You must enable at least one dictionary feature flag to embed a dictionary.

### Web (ES Modules for browsers)

```bash
wasm-pack build --target web --features embed-ipadic
```

### Bundler (Webpack, Vite, Rollup)

```bash
wasm-pack build --target bundler --features embed-ipadic
```

The output is written to the `pkg/` directory inside the `lindera-wasm` crate.

## Available Feature Flags

| Feature | Dictionary | Language |
| --- | --- | --- |
| `embed-ipadic` | IPADIC | Japanese |
| `embed-unidic` | UniDic | Japanese |
| `embed-ko-dic` | ko-dic | Korean |
| `embed-cc-cedict` | CC-CEDICT | Chinese |
| `embed-jieba` | Jieba | Chinese |
| `embed-cjk` | IPADIC + ko-dic + Jieba | CJK (all) |

You can combine multiple dictionaries by enabling multiple feature flags:

```bash
wasm-pack build --target web --features embed-ipadic,embed-ko-dic
```

## NPM Package Naming Convention

When publishing to npm, the recommended naming convention is:

```text
lindera-wasm-{target}
lindera-wasm-{target}-{dict}
```

Examples:

- `lindera-wasm-web`
- `lindera-wasm-web-ipadic`
- `lindera-wasm-bundler-unidic`
- `lindera-wasm-web-cjk`

To set the package name before publishing, edit the `name` field in the generated `pkg/package.json`.

## Installing from npm

If pre-built packages are published to npm:

```bash
npm install lindera-wasm-web-ipadic
```

Or with yarn:

```bash
yarn add lindera-wasm-web-ipadic
```

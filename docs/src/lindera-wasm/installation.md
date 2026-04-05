# Installation

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (v0.10+)

## Obtaining Dictionaries

Lindera WASM does not bundle dictionaries by default. The recommended approach for browser environments is to download dictionaries at runtime using the OPFS (Origin Private File System) API.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. In browser environments, use the OPFS helpers to download and cache dictionaries:

```javascript
import { downloadDictionary, hasDictionary } from 'lindera-wasm-web/opfs';

if (!await hasDictionary("ipadic")) {
    await downloadDictionary(
        "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
        "ipadic",
    );
}
```

See [OPFS Dictionary Storage](./opfs.md) for the full workflow.

## Building with wasm-pack

Build the WASM package for your target environment:

### Web (ES Modules for browsers)

```bash
wasm-pack build --target web
```

### Bundler (Webpack, Vite, Rollup)

```bash
wasm-pack build --target bundler
```

The output is written to the `pkg/` directory inside the `lindera-wasm` crate.

## Available Feature Flags (Advanced)

For advanced users who want to embed dictionaries directly into the WASM binary, the following feature flags are available. This increases the binary size significantly but eliminates the need to download dictionaries at runtime.

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

Pre-built packages are available on npm:

```bash
npm install lindera-wasm-web
```

Or with yarn:

```bash
yarn add lindera-wasm-web
```

> [!NOTE]
> The npm package does not include dictionaries. Use the OPFS helpers to download dictionaries at runtime. See [OPFS Dictionary Storage](./opfs.md).

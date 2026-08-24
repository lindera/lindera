# Installation

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable toolchain)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (v0.10+)

## Obtaining Dictionaries

Lindera WASM does not bundle dictionaries by default. The recommended approach for browser environments is to download dictionaries at runtime using the OPFS (Origin Private File System) API.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. In browser environments, use the OPFS helpers to download and cache dictionaries:

```javascript
import { downloadDictionary, hasDictionary } from 'lindera-wasm/opfs';

if (!await hasDictionary("ipadic")) {
    await downloadDictionary(
        "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
        "ipadic",
    );
}
```

See [OPFS Dictionary Storage](./opfs.md) for the full workflow.

## Building with wasm-pack

Build the WASM package (the published `lindera-wasm` npm package is built with
the `web` target):

```bash
wasm-pack build --target web
```

The output is written to the `pkg/` directory inside the `lindera-wasm` crate.
wasm-pack also supports other targets (e.g. `--target nodejs`) if you need a
custom build, but the web-target build is what this documentation assumes.

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

When publishing a custom build with embedded dictionaries to npm, the
recommended naming convention is:

```text
lindera-wasm-{dict}
```

Examples:

- `lindera-wasm-ipadic`
- `lindera-wasm-unidic`
- `lindera-wasm-cjk`

To set the package name before publishing, edit the `name` field in the generated `pkg/package.json`.

> [!NOTE]
> This project's own release workflow (`.github/workflows/release.yml`) only builds and publishes a single package to npm -- `lindera-wasm`, built with `wasm-pack --target web` and without any `embed-*` feature. Dictionary-suffixed names such as `lindera-wasm-ipadic` are not published anywhere; they only illustrate what a local build with an `embed-*` feature (see [Available Feature Flags](#available-feature-flags-advanced) above) would produce after you rename the package yourself.

## Installing from npm

The pre-built package is available on npm:

```bash
npm install lindera-wasm
```

Or with yarn:

```bash
yarn add lindera-wasm
```

Because the package is a web-target build, call the default-exported async
init function once before using any API:

```javascript
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm';

await __wbg_init();
```

This applies both in browsers (native ES modules) and when the package is
consumed through a bundler -- modern bundlers such as Vite or Webpack 5 (with
the `asyncWebAssembly` experiment) handle the web-target build directly; see
[Browser Usage](./browser_usage.md) for bundler configuration.

> [!NOTE]
> The npm package does not include dictionaries. Use the OPFS helpers to download dictionaries at runtime. See [OPFS Dictionary Storage](./opfs.md).

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

## Why the web Target (and Not bundler)?

wasm-pack's default `--target` is `bundler`, yet the published `lindera-wasm`
package is deliberately built with `--target web`. The target names are
somewhat misleading, so here is what each one actually produces:

- `--target bundler` (the wasm-pack default) emits JavaScript that imports
  the `.wasm` file as an ES module. This relies on the WebAssembly ESM
  integration proposal, which is not yet a finalized web standard; in
  practice the only bundler that understands the output is Webpack with the
  `asyncWebAssembly` experiment enabled. The default dates from the era when
  Webpack was the dominant bundler -- despite the generic name, the output is
  effectively Webpack-specific. Vite, for example, needs extra plugins
  (`vite-plugin-wasm` plus top-level-await support) to consume it.
- `--target web` emits standards-based code only: the `.wasm` file is
  resolved via `new URL('lindera_wasm_bg.wasm', import.meta.url)` and loaded
  with `fetch` + `WebAssembly.instantiateStreaming` inside an explicit async
  init function. This runs in browsers as native ES modules with no build
  step at all, and modern bundlers (Vite, Webpack 5, Rollup) recognize the
  `new URL(..., import.meta.url)` pattern and ship the `.wasm` file as an
  asset. The one cost is that you must call `await __wbg_init()` once before
  using the API.

In short, the bundler target is zero-config for Webpack only, while the web
target runs everywhere at the cost of one explicit init call. Up to v5,
Lindera published both variants (`lindera-wasm-web` and
`lindera-wasm-bundler`); v6 consolidates on the single portable build. If you
are migrating from `lindera-wasm-bundler`, see the
[v5 to v6 migration guide](../migration_v5_to_v6.md).

## Available Feature Flags (Advanced)

For advanced users who want to embed dictionaries directly into the WASM binary, the following feature flags are available. This increases the binary size significantly but eliminates the need to download dictionaries at runtime.

| Feature | Dictionary | Language |
| --- | --- | --- |
| `embed-ipadic` | IPADIC | Japanese |
| `embed-unidic` | UniDic | Japanese |
| `embed-sudachidict` | SudachiDict | Japanese (note: ~570MB, generally impractical for WASM) |
| `embed-ko-dic` | ko-dic | Korean |
| `embed-cc-cedict` | CC-CEDICT | Chinese |
| `embed-jieba` | Jieba | Chinese |
| `embed-cjk` | IPADIC + ko-dic + Jieba | CJK (all) |

You can combine multiple dictionaries by enabling multiple feature flags:

```bash
wasm-pack build --target web --features embed-ipadic,embed-ko-dic
```

## NPM Package Naming Convention

The unscoped `lindera-wasm-*` prefix is the Lindera project's namespace. If you
publish a custom build with embedded dictionaries to npm, use a name under your
own scope so it cannot be mistaken for an official package:

```text
@your-scope/lindera-wasm-{dict}
```

Examples:

- `@your-scope/lindera-wasm-ipadic`
- `@your-scope/lindera-wasm-unidic`
- `@your-scope/lindera-wasm-cjk`

To set the package name before publishing, edit the `name` field in the
generated `pkg/package.json`. Note that a package with an embedded dictionary
redistributes that dictionary, so follow the dictionary's license terms
(attribution and so on) as well.

> [!NOTE]
> This project's own release workflow (`.github/workflows/release.yml`) only builds and publishes a single package to npm -- `lindera-wasm`, built with `wasm-pack --target web` and without any `embed-*` feature. Unscoped dictionary-suffixed names such as `lindera-wasm-ipadic` are reserved by the project (leftovers from pre-v3 releases, no longer updated) and must not be claimed by third parties; in this documentation they only illustrate what a local build with an `embed-*` feature (see [Available Feature Flags](#available-feature-flags-advanced) above) would produce.

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

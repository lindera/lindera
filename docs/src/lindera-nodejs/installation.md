# Installation

## Installing from npm

Pre-built packages will be available on npm:

```bash
npm install lindera
```

> [!NOTE]
> The npm package does not include dictionaries. See [Obtaining Dictionaries](#obtaining-dictionaries) below.
> For browser/WASM usage, see [lindera-wasm](../lindera-wasm/installation.md).

## Building from Source

### Prerequisites

- **Node.js** `^20.17.0 || ^22.13.0 || >= 23.5.0` -- The `engines` range of `lindera-nodejs/package.json` (LTS versions recommended)
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)

The NAPI-RS CLI (`@napi-rs/cli`), which builds the native addon, needs no global install: it is a devDependency of `lindera-nodejs`, which `npm install` installs (see [Development Build](#development-build)).

## Obtaining Dictionaries

Lindera does not bundle dictionaries with the package. You need to obtain a pre-built dictionary separately.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. Download and extract the dictionary archive to a local directory:

```bash
# Example: download and extract the IPADIC dictionary
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## Development Build

Build lindera-nodejs in release mode. `npm install` also installs the NAPI-RS CLI (`@napi-rs/cli`) at the version `package.json` pins, which `npm run build` runs:

```bash
cd lindera-nodejs
npm install
npm run build
```

For a debug build, run `npm run build:debug` instead of `npm run build`.

Or use the project Makefile, which makes the same release build:

```bash
make build-lindera-nodejs
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality. It is enabled by default:

```bash
npm run build -- --features train
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) into the binary | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) into the binary | Disabled |
| `embed-sudachidict` | Embed Japanese dictionary (SudachiDict) into the binary | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) into the binary | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) into the binary | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) into the binary | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) into the binary | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) into the binary | Disabled |

Multiple features can be combined:

```bash
npm run build -- --features "train,embed-ipadic,embed-ko-dic"
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```javascript
> const dictionary = loadDictionary("embedded://ipadic");
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Verifying the Installation

After installation, verify that lindera is available in Node.js:

```javascript
const lindera = require("lindera");

console.log(lindera.version());
```

> [!NOTE]
> The `lindera` npm package's `exports` map declares `types`, `import`, and `require`
> conditions, so the package loads from CommonJS
> (`const { TokenizerBuilder } = require("lindera")`) and ES modules
> (`import { TokenizerBuilder } from "lindera"`) alike:

```javascript
import { TokenizerBuilder } from "lindera";

console.log(typeof TokenizerBuilder); // "function"
```

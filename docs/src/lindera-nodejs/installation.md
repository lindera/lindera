# Installation

## Installing from npm

Pre-built packages will be available on npm:

```bash
npm install lindera-nodejs
```

> [!NOTE]
> The npm package does not include dictionaries. See [Obtaining Dictionaries](#obtaining-dictionaries) below.
> For browser/WASM usage, see [lindera-wasm](../lindera-wasm/installation.md).

## Building from Source

### Prerequisites

- **Node.js 18 or later** (LTS versions recommended)
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **NAPI-RS CLI** -- CLI tool for building native Node.js addons in Rust

Install the NAPI-RS CLI globally:

```bash
npm install -g @napi-rs/cli
```

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

Build lindera-nodejs in development mode:

```bash
cd lindera-nodejs
npm install
npm run build
```

Or use the project Makefile:

```bash
make nodejs-develop
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
const lindera = require("lindera-nodejs");

console.log(lindera.version());
```

Or with ES modules:

```javascript
import { version } from "lindera-nodejs";

console.log(version());
```

# Installation

## Prerequisites

- **Node.js 18 or later** (LTS versions recommended)
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **NAPI-RS CLI** -- CLI tool for building native Node.js addons in Rust

Install the NAPI-RS CLI globally:

```bash
npm install -g @napi-rs/cli
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

### Build with Embedded Dictionaries

Embed dictionaries directly into the binary so no external dictionary files are needed at runtime:

```bash
npm run build -- --features embed-ipadic
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) | Disabled |

Multiple features can be combined:

```bash
npm run build -- --features "train,embed-ipadic,embed-ko-dic"
```

## Verifying the Installation

After installation, verify that lindera is available in Node.js:

```javascript
const lindera = require("lindera");

console.log(lindera.version());
```

Or with ES modules:

```javascript
import { version } from "lindera";

console.log(version());
```

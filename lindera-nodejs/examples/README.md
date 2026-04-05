# Lindera Node.js Examples

## Prerequisites

- Node.js (v18+)
- Rust toolchain
- NAPI CLI (`npm install -g @napi-rs/cli`)

All commands below should be run from the repository root (`lindera/`) unless otherwise noted.

## Build

Install npm dependencies and build the native module:

```bash
cd lindera-nodejs
npm install

# Build with embedded IPADIC dictionary (debug)
npx napi build --platform -p lindera-nodejs --features embed-ipadic

# Build with training feature (debug)
npx napi build --platform -p lindera-nodejs --features embed-ipadic,train

# Release build
npx napi build --platform --release -p lindera-nodejs --features embed-ipadic
```

## Examples

Run from the `lindera-nodejs/` directory after building:

```bash
cd lindera-nodejs
```

### Basic tokenization

```bash
node examples/tokenize.js
```

Tokenizes Japanese text using the embedded IPADIC dictionary in "normal" mode.

### Decompose mode

```bash
node examples/tokenize_with_decompose.js
```

Tokenizes text in "decompose" mode, which decomposes compound morphemes.

### Tokenization with filters

```bash
node examples/tokenize_with_filters.js
```

Demonstrates `TokenizerBuilder` API with character filters (`unicode_normalize`, `japanese_iteration_mark`, `mapping`) and token filters (`japanese_katakana_stem`, `japanese_stop_tags`, `lowercase`, `japanese_base_form`).

### User dictionary

```bash
node examples/tokenize_with_userdict.js
```

Loads a custom user dictionary from `resources/ipadic_simple_userdic.csv` alongside the standard dictionary.

### Build dictionary from source

```bash
node examples/build_ipadic.js
```

Downloads the mecab-ipadic source tarball and builds a Lindera dictionary from it. Requires `tar` command.

### Train and export (requires `train` feature)

```bash
# Build with train feature first
npx napi build --platform -p lindera-nodejs --features embed-ipadic,train

node examples/train_and_export.js
```

Trains a CRF model from a sample corpus and exports dictionary files.

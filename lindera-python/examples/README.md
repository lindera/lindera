# Lindera Python Examples

## Prerequisites

- Python >= 3.10
- Rust toolchain
- [Maturin](https://www.maturin.rs/) (`pip install maturin`)

## Build

Run from the `lindera-python/` directory:

```bash
cd lindera-python

# Build with embedded IPADIC dictionary
maturin develop --features embed-ipadic

# Build with training feature
maturin develop --features embed-ipadic,train

# Release build
maturin develop --release --features embed-ipadic
```

## Examples

Run from the `lindera-python/` directory after building:

### Basic tokenization

```bash
python examples/tokenize.py
```

Tokenizes Japanese text using the embedded IPADIC dictionary in "normal" mode.

### Decompose mode

```bash
python examples/tokenize_with_decompose.py
```

Tokenizes text in "decompose" mode, which decomposes compound morphemes.

### Tokenization with filters

```bash
python examples/tokenize_with_filters.py
```

Demonstrates `TokenizerBuilder` API with character filters (`unicode_normalize`, `japanese_iteration_mark`, `mapping`) and token filters (`japanese_katakana_stem`, `japanese_stop_tags`, `lowercase`, `japanese_base_form`).

### User dictionary

```bash
python examples/tokenize_with_userdict.py
```

Loads a custom user dictionary from `resources/ipadic_simple_userdic.csv` alongside the standard dictionary.

### Build dictionary from source

```bash
python examples/build_ipadic.py
```

Downloads the mecab-ipadic source tarball and builds a Lindera dictionary from it.

### Train and export (requires `train` feature)

```bash
# Build with train feature first
maturin develop --features embed-ipadic,train

python examples/train_and_export.py
```

Trains a CRF model from a sample corpus and exports dictionary files.

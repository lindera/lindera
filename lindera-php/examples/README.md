# Lindera PHP Examples

## Prerequisites

- PHP >= 8.1
- Rust toolchain
- Composer (`composer install` in `lindera-php/`)
- `ext-php-rs` compatible environment (see [ext-php-rs docs](https://github.com/davidcole1340/ext-php-rs))

## Build

Build the PHP extension from the repository root (`lindera/`):

```bash
# Build with embedded IPADIC dictionary (debug)
cargo build -p lindera-php --features embed-ipadic

# Build with training feature (debug)
cargo build -p lindera-php --features embed-ipadic,train

# Release build
cargo build -p lindera-php --release --all-features
```

The extension will be at `target/debug/liblindera_php.so` (or `target/release/liblindera_php.so` for release builds).

## Examples

Run from the `lindera-php/` directory:

```bash
cd lindera-php
```

### Basic tokenization

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize.php
```

Tokenizes Japanese text using the embedded IPADIC dictionary in "normal" mode.

### Decompose mode

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_decompose.php
```

Tokenizes text in "decompose" mode, which decomposes compound morphemes.

### Tokenization with filters

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_filters.php
```

Demonstrates `TokenizerBuilder` API with character filters (`unicode_normalize`, `japanese_iteration_mark`, `mapping`) and token filters (`japanese_katakana_stem`, `japanese_stop_tags`, `lowercase`, `japanese_base_form`).

### User dictionary

```bash
php -d extension=../target/debug/liblindera_php.so examples/tokenize_with_userdict.php
```

Loads a custom user dictionary from `resources/ipadic_simple_userdic.csv` alongside the standard dictionary.

### Build dictionary from source

```bash
php -d extension=../target/debug/liblindera_php.so examples/build_ipadic.php
```

Downloads the mecab-ipadic source tarball and builds a Lindera dictionary from it.

### Train and export (requires `train` feature)

```bash
# Build with train feature first (from repository root)
cargo build -p lindera-php --features embed-ipadic,train

php -d extension=../target/debug/liblindera_php.so examples/train_and_export.php
```

Trains a CRF model from a sample corpus and exports dictionary files.

### Web application

```bash
php -d extension=../target/debug/liblindera_php.so -S localhost:8080 examples/tokenize_app.php
```

Then open <http://localhost:8080> in your browser. Provides an interactive web UI for morphological analysis with mode selection (normal/decompose).

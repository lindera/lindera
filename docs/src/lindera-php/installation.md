# Installation

## Prerequisites

- **PHP 8.1 or later**
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **Composer** -- PHP dependency manager (optional, for project integration)

## Development Build

Build the lindera-php extension from the project root:

```bash
cargo build -p lindera-php
```

Or use the project Makefile:

```bash
make php-build
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality:

```bash
cargo build -p lindera-php --features train
```

### Build with Embedded Dictionaries

Embed dictionaries directly into the shared library so no external dictionary files are needed at runtime:

```bash
cargo build -p lindera-php --features embed-ipadic
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Disabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) | Disabled |

Multiple features can be combined:

```bash
cargo build -p lindera-php --features "train,embed-ipadic,embed-ko-dic"
```

## Loading the Extension

Load the compiled shared library when running PHP:

```bash
php -d extension=target/debug/liblindera_php.so script.php
```

For release builds:

```bash
cargo build -p lindera-php --release --features embed-ipadic
php -d extension=target/release/liblindera_php.so script.php
```

Alternatively, add the extension to your `php.ini`:

```ini
extension=/absolute/path/to/liblindera_php.so
```

## Verifying the Installation

After building, verify that lindera is available in PHP:

```bash
php -d extension=target/debug/liblindera_php.so -r "echo Lindera\Dictionary::version() . PHP_EOL;"
```

# Installation

> [!NOTE]
> lindera-php is not yet published to Packagist. You need to build from source.

## Prerequisites

- **PHP 8.1 or later**
- **Rust toolchain** -- Install via [rustup](https://rustup.rs/)
- **Composer** -- PHP dependency manager (optional, for running tests)

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

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Disabled |
| `embed-ipadic` | Embed Japanese dictionary (IPADIC) into the binary | Disabled |
| `embed-unidic` | Embed Japanese dictionary (UniDic) into the binary | Disabled |
| `embed-ipadic-neologd` | Embed Japanese dictionary (IPADIC NEologd) into the binary | Disabled |
| `embed-ko-dic` | Embed Korean dictionary (ko-dic) into the binary | Disabled |
| `embed-cc-cedict` | Embed Chinese dictionary (CC-CEDICT) into the binary | Disabled |
| `embed-jieba` | Embed Chinese dictionary (Jieba) into the binary | Disabled |
| `embed-cjk` | Embed all CJK dictionaries (IPADIC, ko-dic, Jieba) into the binary | Disabled |

Multiple features can be combined:

```bash
cargo build -p lindera-php --features "train,embed-ipadic,embed-ko-dic"
```

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```php
> $dictionary = Lindera\Dictionary::load('embedded://ipadic');
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Loading the Extension

Load the compiled shared library when running PHP:

```bash
php -d extension=target/debug/liblindera_php.so script.php
```

For release builds:

```bash
cargo build -p lindera-php --release
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

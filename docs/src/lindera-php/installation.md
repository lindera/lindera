# Installation

lindera-php is a PHP extension. It is published on [Packagist](https://packagist.org/packages/lindera/lindera) as `lindera/lindera` and installed with [PIE](https://github.com/php/pie), the PHP extension installer. The extension registers as `lindera`.

## Prerequisites

- **PHP 8.1 or later** on Linux or macOS. Windows is not supported.
- **PIE** -- see the [PIE installation guide](https://github.com/php/pie/blob/main/docs/usage.md)
- For building from source (which is what PIE does today): a **Rust toolchain** ([rustup](https://rustup.rs/)), **libclang** (`libclang-dev` on Debian/Ubuntu; the Xcode command line tools or `brew install llvm` on macOS) and the usual PHP extension build tools (`autoconf`, `libtool`, `make`)

## Install with PIE

```bash
pie install lindera/lindera
```

PIE downloads the source, builds the extension against the PHP it finds (pass `--with-php-config=/path/to/php-config` to target another one), installs `lindera.so` into the extension directory and enables it. Composer itself does not install `php-ext` packages, so `composer require lindera/lindera` is not an installation route.

> [!NOTE]
> `lindera/lindera` is available on Packagist from lindera v6.1.0. For earlier versions, build from source as described below.

The package does not embed any dictionary; see [Obtaining Dictionaries](#obtaining-dictionaries).

## Obtaining Dictionaries

Lindera does not bundle dictionaries with the package. You need to obtain a pre-built dictionary separately.

### Download from GitHub Releases

Pre-built dictionaries are available on the [GitHub Releases](https://github.com/lindera/lindera/releases) page. Download and extract the dictionary archive to a local directory:

```bash
# Example: download and extract the IPADIC dictionary
curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

## Building from Source

Build the lindera-php extension from the project root:

```bash
cargo build -p lindera-php
```

Or use the project Makefile:

```bash
make build-lindera-php
```

### Build with Training Support

The `train` feature enables CRF-based dictionary training functionality:

```bash
cargo build -p lindera-php --features train
```

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `train` | CRF training functionality | Enabled (default) |
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
cargo build -p lindera-php --features "train,embed-ipadic,embed-ko-dic"
```

PIE builds with the default features only (`train`, no embedded dictionary).

> [!TIP]
> If you want to embed a dictionary directly into the binary (advanced usage), enable the corresponding `embed-*` feature flag and load it using the `embedded://` scheme:
>
> ```php
> $dictionary = Lindera\Dictionary::load('embedded://ipadic');
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

## Loading a Self-Built Extension

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

On macOS, cargo produces `liblindera_php.dylib` instead. Whichever way it is loaded, the extension registers as `lindera`: `php -m` lists `lindera`, and `extension_loaded('lindera')` is the check to use.

## Verifying the Installation

Verify that lindera is available in PHP:

```bash
# Installed with PIE
php -r "echo Lindera\Dictionary::version() . PHP_EOL;"

# Self-built
php -d extension=target/debug/liblindera_php.so -r "echo Lindera\Dictionary::version() . PHP_EOL;"
```

## Stubs for Static Analysis

[`lindera-php/stubs/lindera.stubs.php`](https://github.com/lindera/lindera/blob/main/lindera-php/stubs/lindera.stubs.php) declares every class of the extension for IDEs, PHPStan and Psalm. It is generated from the compiled extension and the test suite fails when it is out of date. Point your analyser at a copy of the file, and never include it at runtime -- the extension already declares the classes.

```neon
# phpstan.neon
parameters:
    stubFiles:
        - path/to/lindera.stubs.php
```

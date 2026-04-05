# Installation

## Install via Cargo

You can install the binary via cargo:

```shell
% cargo install lindera-cli
```

## Download from GitHub Releases

Alternatively, you can download a pre-built binary from the release page:

- [https://github.com/lindera/lindera/releases](https://github.com/lindera/lindera/releases)

## Obtaining Dictionaries

Lindera does not bundle dictionaries with the binary. You need to download a pre-built dictionary separately from the [GitHub Releases](https://github.com/lindera/lindera/releases) page:

```shell
# Example: download and extract the IPADIC dictionary
% curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
% unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

Then specify the dictionary path when using the CLI:

```shell
% lindera tokenize --dictionary /path/to/ipadic "関西国際空港限定トートバッグ"
```

## Build from Source

### Build without dictionaries (default)

Build a binary containing only the tokenizer and trainer without embedded dictionaries:

```shell
% cargo build --release
```

### Build with all features

```shell
% cargo build --release --all-features
```

### Build with Embedded Dictionaries (Advanced)

For advanced users who want to embed dictionaries directly into the binary, use the `embed-*` feature flags. This eliminates the need for external dictionary files at runtime but increases the binary size.

#### IPADIC (Japanese dictionary)

```shell
% cargo build --release --features=embed-ipadic
```

#### IPADIC NEologd (Japanese dictionary)

```shell
% cargo build --release --features=embed-ipadic-neologd
```

#### UniDic (Japanese dictionary)

```shell
% cargo build --release --features=embed-unidic
```

#### ko-dic (Korean dictionary)

```shell
% cargo build --release --features=embed-ko-dic
```

#### CC-CEDICT (Chinese dictionary)

```shell
% cargo build --release --features=embed-cc-cedict
```

#### Jieba (Chinese dictionary)

```shell
% cargo build --release --features=embed-jieba
```

> [!TIP]
> After building with an `embed-*` feature flag, use the `embedded://` scheme to load the embedded dictionary:
>
> ```shell
> % lindera tokenize --dictionary embedded://ipadic "関西国際空港限定トートバッグ"
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

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

Lindera does not bundle dictionaries with the binary. The easiest way to obtain one is the `download` subcommand, which fetches the pre-built dictionary matching the CLI version and installs it under the OS-standard application data directory:

```shell
% lindera download ipadic
```

After downloading, the dictionary can be referenced by name:

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dict ipadic
```

See [Commands](commands.md#download) for the available dictionary names and storage locations.

Alternatively, you can download a pre-built dictionary manually from the [GitHub Releases](https://github.com/lindera/lindera/releases) page:

```shell
# Example: download and extract the IPADIC dictionary
% curl -LO https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip
% unzip lindera-ipadic-<version>.zip -d /path/to/ipadic
```

Then specify the extracted dictionary path when using the CLI (note that the archive contains a `lindera-ipadic` directory):

```shell
% echo "関西国際空港限定トートバッグ" | lindera tokenize --dict /path/to/ipadic/lindera-ipadic
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

#### SudachiDict (Japanese dictionary)

```shell
% cargo build --release --features=embed-sudachidict
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
> % echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic
> ```
>
> See [Feature Flags](../development/feature_flags.md) for details.

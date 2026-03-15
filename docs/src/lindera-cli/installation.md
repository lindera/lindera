# Installation

## Install via Cargo

You can install the binary via cargo:

```shell
% cargo install lindera-cli
```

## Download from GitHub Releases

Alternatively, you can download a pre-built binary from the release page:

- [https://github.com/lindera/lindera/releases](https://github.com/lindera/lindera/releases)

## Build from Source

### Build with IPADIC (Japanese dictionary)

The "ipadic" feature flag allows Lindera to include IPADIC.

```shell
% cargo build --release --features=embed-ipadic
```

### Build with UniDic (Japanese dictionary)

The "unidic" feature flag allows Lindera to include UniDic.

```shell
% cargo build --release --features=embed-unidic
```

### Build with ko-dic (Korean dictionary)

The "ko-dic" feature flag allows Lindera to include ko-dic.

```shell
% cargo build --release --features=embed-ko-dic
```

### Build with CC-CEDICT (Chinese dictionary)

The "cc-cedict" feature flag allows Lindera to include CC-CEDICT.

```shell
% cargo build --release --features=embed-cc-cedict
```

### Build with Jieba (Chinese dictionary)

The "jieba" feature flag allows Lindera to include Jieba.

```shell
% cargo build --release --features=embed-jieba
```

### Build without dictionaries

To reduce Lindera's binary size, omit the feature flag.
This results in a binary containing only the tokenizer and trainer, as it no longer includes the dictionary.

```shell
% cargo build --release
```

### Build with all features

```shell
% cargo build --release --all-features
```

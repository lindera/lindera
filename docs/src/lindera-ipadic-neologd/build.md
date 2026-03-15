# Build

This page describes how to build the IPADIC NEologd dictionary from source files.

## Build system dictionary

Download the IPADIC NEologd source files and build the dictionary:

```shell
% curl -L -o /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"
% tar zxvf /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp

% lindera build \
  --src /tmp/mecab-ipadic-neologd-0.0.7-20200820 \
  --dest /tmp/lindera-ipadic-neologd-0.0.7-20200820 \
  --metadata ./lindera-ipadic-neologd/metadata.json
```

## Embedding in binary

To embed the IPADIC NEologd dictionary directly into the binary:

```shell
cargo build --features=embed-ipadic-neologd
```

This allows using `embedded://ipadic-neologd` as the dictionary path without external dictionary files.

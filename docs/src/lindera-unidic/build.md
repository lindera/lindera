# Build

This page describes how to build the UniDic dictionary from source files.

## Build system dictionary

Download the UniDic source files and build the dictionary:

```shell
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxvf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp

% lindera build \
  --src /tmp/unidic-mecab-2.1.2 \
  --dest /tmp/lindera-unidic-2.1.2 \
  --metadata ./lindera-unidic/metadata.json
```

## Build user dictionary

Build a user dictionary from a CSV file:

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the UniDic dictionary directly into the binary:

```shell
cargo build --features=embed-unidic
```

This allows using `embedded://unidic` as the dictionary path without external dictionary files.

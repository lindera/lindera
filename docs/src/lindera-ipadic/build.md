# Build

This page describes how to build the IPADIC dictionary from source files.

## Build system dictionary

Download the IPADIC source files and build the dictionary:

```shell
# Download and extract IPADIC source files
% curl -L -o /tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C /tmp

# Build the dictionary
% lindera build \
  --src /tmp/mecab-ipadic-2.7.0-20250920 \
  --dest /tmp/lindera-ipadic-2.7.0-20250920 \
  --metadata ./lindera-ipadic/metadata.json
```

## Build user dictionary

Build a user dictionary from a CSV file:

```shell
% lindera build \
  --src ./resources/user_dict/ipadic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the IPADIC dictionary directly into the binary:

```shell
cargo build --features=embed-ipadic
```

This allows using `embedded://ipadic` as the dictionary path without external dictionary files.

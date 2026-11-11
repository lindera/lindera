# Build

This page describes how to build the UniDic CWJ dictionary from source files.

## Build system dictionary

Download NINJAL's full package, extract the four files the builder reads, and build the dictionary. The lighter package, `unidic-cwj-202512.zip`, holds only MeCab's compiled files and cannot be built from.

```shell
% curl -L -o /tmp/unidic-cwj-202512_full.zip "https://clrd.ninjal.ac.jp/unidic_archive/2512/unidic-cwj-202512_full.zip"
% shasum -a 256 /tmp/unidic-cwj-202512_full.zip
ca02ad51c8ae238373dcb150eec014f1ad9f05f54c7416a16c2fe444fd4921b3  /tmp/unidic-cwj-202512_full.zip
% unzip /tmp/unidic-cwj-202512_full.zip lex.csv matrix.def char.def unk.def -d /tmp/unidic-cwj-202512

% lindera build \
  --src /tmp/unidic-cwj-202512 \
  --dest /tmp/lindera-unidic-cwj-202512 \
  --metadata ./lindera-unidic-cwj/metadata.json
```

The package is 2.77 GiB, and the four files take 7.95 GiB, `matrix.def` 7.70 GiB of them. The builder reads `matrix.def` whole, so the build takes at least that much memory.

`lindera-unidic-cwj/metadata.json` sets `connection_id_mapping: false`, and the crate ships no `context_id_freq.txt`, so no `--context-id-freq` is needed.

## Build user dictionary

Build a user dictionary from a CSV file. The simple format is the same as UniDic 2.1.2's:

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic-cwj/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the UniDic CWJ dictionary directly into the binary:

```shell
cargo build --features=embed-unidic-cwj
```

This allows using `embedded://unidic-cwj` as the dictionary path without external dictionary files. The binary then carries the 1.2 GB dictionary, and compiling it takes about 11 GiB of memory.

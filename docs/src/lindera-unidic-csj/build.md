# Build

This page describes how to build the UniDic CSJ dictionary from source files.

## Build system dictionary

Download NINJAL's full package, extract the four files the builder reads, and build the dictionary. The lighter package, `unidic-csj-202512.zip`, holds only MeCab's compiled files and cannot be built from.

```shell
% curl -L -o /tmp/unidic-csj-202512_full.zip "https://clrd.ninjal.ac.jp/unidic_archive/2512/unidic-csj-202512_full.zip"
% shasum -a 256 /tmp/unidic-csj-202512_full.zip
f737d26ce9daf96347e1ccaf34896a36651ffb7ceff91d4d8041591ff7a387e4  /tmp/unidic-csj-202512_full.zip
% unzip /tmp/unidic-csj-202512_full.zip lex.csv matrix.def char.def unk.def -d /tmp/unidic-csj-202512

% lindera build \
  --src /tmp/unidic-csj-202512 \
  --dest /tmp/lindera-unidic-csj-202512 \
  --metadata ./lindera-unidic-csj/metadata.json \
  --context-id-freq ./lindera-unidic-csj/context_id_freq.txt
```

The package is 2.81 GiB, and the four files take 8.18 GiB, `matrix.def` 7.92 GiB of them. The builder reads `matrix.def` in blocks of 64 MiB, so it holds the 1,002 MiB matrix and one block rather than the whole file: building the dictionary allocated at most 2.50 GiB on macOS.

`lindera-unidic-csj/metadata.json` is UniDic CWJ's with another name: the two packages have the same fields. It sets `connection_id_mapping: true`, and the crate ships its own `context_id_freq.txt`, counted over `resources/bocchan.txt` as UniDic CWJ's is: pass it with `--context-id-freq` / `-f`. See [UniDic CWJ](../lindera-unidic-cwj/build.md) for what the relabeling does.

## Build user dictionary

Build a user dictionary from a CSV file. The simple format is the same as UniDic 2.1.2's:

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic-csj/metadata.json \
  --user
```

For more details about user dictionary format, see [Dictionary Format](./dictionary_format.md).

## Embedding in binary

To embed the UniDic CSJ dictionary directly into the binary:

```shell
cargo build --features=embed-unidic-csj
```

This allows using `embedded://unidic-csj` as the dictionary path without external dictionary files. The binary then carries the 1.2 GB dictionary, and compiling it takes 8 to 11 GiB of memory, as for UniDic CWJ.

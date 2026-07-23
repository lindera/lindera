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
  --metadata ./lindera-unidic/metadata.json \
  --context-id-freq ./lindera-unidic/context_id_freq.txt
```

> [!TIP]
> `lindera-unidic/metadata.json` sets `connection_id_mapping: true`, so the builder relabels
> the connection-cost matrix's context IDs by access frequency to improve cache locality when
> looking up connection costs. Passing `--context-id-freq` / `-f` with the bundled
> `context_id_freq.txt` histogram gives this remapping real corpus frequency data to rank IDs
> by. Omitting the flag silently falls back to a much weaker entry-count-based proxy instead of
> failing, so the build still succeeds but without the full benefit. Either way, tokenization
> output is unaffected -- the remap is a bijective relabeling that only changes a build-time
> optimization, never correctness.

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

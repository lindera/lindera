# Build

This page describes how to build the UniDic dictionary from source files.

## Build system dictionary

Download NINJAL's source package of UniDic 2.1.2 and build the dictionary:

```shell
% curl -L -o /tmp/unidic-mecab-2.1.2_src.zip "https://clrd.ninjal.ac.jp/unidic_archive/cwj/2.1.2/unidic-mecab-2.1.2_src.zip"
% shasum -a 256 /tmp/unidic-mecab-2.1.2_src.zip
6cce98269214ce7de6159f61a25ffc5b436375c098cc86d6aa98c0605cbf90d4  /tmp/unidic-mecab-2.1.2_src.zip
% unzip /tmp/unidic-mecab-2.1.2_src.zip -d /tmp

% lindera build \
  --src /tmp/unidic-mecab-2.1.2_src \
  --dest /tmp/lindera-unidic-2.1.2 \
  --metadata ./lindera-unidic/metadata.json \
  --context-id-freq ./lindera-unidic/context_id_freq.txt
```

> [!TIP]
> `lindera-unidic/metadata.json` sets `connection_id_mapping: true`, so the builder relabels
> the connection-cost matrix's context IDs by access frequency to improve cache locality when
> looking up connection costs. Passing `--context-id-freq` / `-f` with the bundled
> `context_id_freq.txt` histogram gives this remapping real corpus frequency data to rank IDs
> by. Without the flag, the build still succeeds, but falls back to a much weaker
> entry-count-based proxy and writes this warning to standard error:
>
> ```text
> [WARN  lindera_dictionary::builder::context_id_remap] connection_id_mapping is enabled but no context-id frequency file was found; falling back to the entry-count proxy, which measured ~0% improvement. Bundle a corpus-derived histogram (see the ctxfreq_dump example) for the real gain.
> ```
>
> Either way, tokenization output is unaffected -- the remap is a bijective relabeling that
> only changes a build-time optimization, never correctness.

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

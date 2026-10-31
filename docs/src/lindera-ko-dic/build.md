# Build

## Build system dictionary

Download and extract the mecab-ko-dic source files, then build the dictionary:

```shell
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera build \
  --src /tmp/mecab-ko-dic-2.1.1-20180720 \
  --dest /tmp/lindera-ko-dic-2.1.1-20180720 \
  --metadata ./lindera-ko-dic/metadata.json \
  --context-id-freq ./lindera-ko-dic/context_id_freq.txt
```

> [!TIP]
> `lindera-ko-dic/metadata.json` sets `connection_id_mapping: true`, so the builder relabels
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

```shell
% lindera build \
  --src ./resources/user_dict/ko-dic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ko-dic/metadata.json \
  --user
```

## Embedding the dictionary

To embed the ko-dic dictionary directly into the binary, build with the following feature flag:

```shell
% cargo build --features=embed-ko-dic
```

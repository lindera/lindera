# Build

## Build system dictionary

Download and extract the CC-CEDICT-MeCab source files, then build the dictionary:

```shell
% curl -L -o /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz "https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
% tar zxvf /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz -C /tmp
% lindera build \
  --src /tmp/CC-CEDICT-MeCab-0.1.0-20200409 \
  --dest /tmp/lindera-cc-cedict-0.1.0-20200409 \
  --metadata ./lindera-cc-cedict/metadata.json
```

## Build user dictionary

```shell
% lindera build \
  --src ./resources/user_dict/cc-cedict_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-cc-cedict/metadata.json \
  --user
```

## Embedding the dictionary

To embed the CC-CEDICT dictionary directly into the binary, build with the following feature flag:

```shell
% cargo build --features=embed-cc-cedict
```

# Build

## Build system dictionary

Download and extract the mecab-jieba source files, then build the dictionary:

```shell
% curl -L -o /tmp/mecab-jieba-0.1.0-20260310.tar.gz "https://lindera.dev/mecab-jieba-0.1.0-20260310.tar.gz"
% tar zxvf /tmp/mecab-jieba-0.1.0-20260310.tar.gz -C /tmp
% lindera build \
  --src /tmp/mecab-jieba-0.1.0-20260310 \
  --dest /tmp/lindera-jieba-0.1.0-20260310 \
  --metadata ./lindera-jieba/metadata.json
```

## Build user dictionary

```shell
% lindera build \
  --src ./resources/user_dict/jieba_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-jieba/metadata.json \
  --user
```

## Embedding the dictionary

To embed the Jieba dictionary directly into the binary, build with the following feature flag:

```shell
% cargo build --features=embed-jieba
```

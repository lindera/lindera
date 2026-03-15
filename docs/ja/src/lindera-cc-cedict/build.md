# ビルド

## システム辞書のビルド

CC-CEDICT-MeCab のソースファイルをダウンロード・展開し、辞書をビルドします:

```shell
% curl -L -o /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz "https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
% tar zxvf /tmp/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz -C /tmp
% lindera build \
  --src /tmp/CC-CEDICT-MeCab-0.1.0-20200409 \
  --dest /tmp/lindera-cc-cedict-0.1.0-20200409 \
  --metadata ./lindera-cc-cedict/metadata.json
```

## ユーザー辞書のビルド

```shell
% lindera build \
  --src ./resources/user_dict/cc-cedict_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-cc-cedict/metadata.json \
  --user
```

## 辞書の埋め込み

CC-CEDICT 辞書をバイナリに直接埋め込むには、以下の feature フラグを付けてビルドします:

```shell
% cargo build --features=embed-cc-cedict
```

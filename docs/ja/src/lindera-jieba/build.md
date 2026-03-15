# ビルド

## システム辞書のビルド

mecab-jieba のソースファイルをダウンロード・展開し、辞書をビルドします:

```shell
% curl -L -o /tmp/mecab-jieba-0.1.0-20260310.tar.gz "https://lindera.dev/mecab-jieba-0.1.0-20260310.tar.gz"
% tar zxvf /tmp/mecab-jieba-0.1.0-20260310.tar.gz -C /tmp
% lindera build \
  --src /tmp/mecab-jieba-0.1.0-20260310 \
  --dest /tmp/lindera-jieba-0.1.0-20260310 \
  --metadata ./lindera-jieba/metadata.json
```

## ユーザー辞書のビルド

```shell
% lindera build \
  --src ./resources/user_dict/jieba_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-jieba/metadata.json \
  --user
```

## 辞書の埋め込み

Jieba 辞書をバイナリに直接埋め込むには、以下の feature フラグを付けてビルドします:

```shell
% cargo build --features=embed-jieba
```

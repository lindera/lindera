# ビルド

## システム辞書のビルド

mecab-ko-dic のソースファイルをダウンロード・展開し、辞書をビルドします:

```shell
% curl -L -o /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz "https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
% tar zxvf /tmp/mecab-ko-dic-2.1.1-20180720.tar.gz -C /tmp
% lindera build \
  --src /tmp/mecab-ko-dic-2.1.1-20180720 \
  --dest /tmp/lindera-ko-dic-2.1.1-20180720 \
  --metadata ./lindera-ko-dic/metadata.json
```

## ユーザー辞書のビルド

```shell
% lindera build \
  --src ./resources/user_dict/ko-dic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ko-dic/metadata.json \
  --user
```

## 辞書の埋め込み

ko-dic 辞書をバイナリに直接埋め込むには、以下の feature フラグを付けてビルドします:

```shell
% cargo build --features=embed-ko-dic
```

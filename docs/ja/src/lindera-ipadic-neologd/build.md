# ビルド

このページでは、ソースファイルから IPADIC NEologd 辞書をビルドする方法を説明します。

## システム辞書のビルド

IPADIC NEologd のソースファイルをダウンロードし、辞書をビルドします:

```shell
% curl -L -o /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"
% tar zxvf /tmp/mecab-ipadic-neologd-0.0.7-20200820.tar.gz -C /tmp

% lindera build \
  --src /tmp/mecab-ipadic-neologd-0.0.7-20200820 \
  --dest /tmp/lindera-ipadic-neologd-0.0.7-20200820 \
  --metadata ./lindera-ipadic-neologd/metadata.json
```

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします:

```shell
% lindera build \
  --src ./resources/user_dict/ipadic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ipadic-neologd/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

IPADIC NEologd 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-ipadic-neologd
```

これにより、外部辞書ファイルなしで `embedded://ipadic-neologd` を辞書パスとして使用できるようになります。

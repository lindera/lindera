# ビルド

このページでは、ソースファイルから IPADIC 辞書をビルドする方法を説明します。

## システム辞書のビルド

IPADIC のソースファイルをダウンロードし、辞書をビルドします:

```shell
# IPADIC ソースファイルのダウンロードと展開
% curl -L -o /tmp/mecab-ipadic-2.7.0-20250920.tar.gz "https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"
% tar zxvf /tmp/mecab-ipadic-2.7.0-20250920.tar.gz -C /tmp

# 辞書のビルド
% lindera build \
  --src /tmp/mecab-ipadic-2.7.0-20250920 \
  --dest /tmp/lindera-ipadic-2.7.0-20250920 \
  --metadata ./lindera-ipadic/metadata.json
```

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします:

```shell
% lindera build \
  --src ./resources/user_dict/ipadic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-ipadic/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

IPADIC 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-ipadic
```

これにより、外部辞書ファイルなしで `embedded://ipadic` を辞書パスとして使用できるようになります。

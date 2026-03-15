# ビルド

このページでは、ソースファイルから UniDic 辞書をビルドする方法を説明します。

## システム辞書のビルド

UniDic のソースファイルをダウンロードし、辞書をビルドします:

```shell
% curl -L -o /tmp/unidic-mecab-2.1.2.tar.gz "https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"
% tar zxvf /tmp/unidic-mecab-2.1.2.tar.gz -C /tmp

% lindera build \
  --src /tmp/unidic-mecab-2.1.2 \
  --dest /tmp/lindera-unidic-2.1.2 \
  --metadata ./lindera-unidic/metadata.json
```

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします:

```shell
% lindera build \
  --src ./resources/user_dict/unidic_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-unidic/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

UniDic 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-unidic
```

これにより、外部辞書ファイルなしで `embedded://unidic` を辞書パスとして使用できるようになります。

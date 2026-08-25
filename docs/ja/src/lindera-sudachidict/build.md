# ビルド

このページでは、ソースファイルから SudachiDict 辞書をビルドする方法を説明します。

## システム辞書のビルド

SudachiDict のソースアーカイブをダウンロードし、辞書をビルドします:

```shell
% curl -L -o /tmp/sudachidict-20260723.tar.gz "https://lindera.dev/sudachidict-20260723.tar.gz"
% tar zxvf /tmp/sudachidict-20260723.tar.gz -C /tmp

% lindera build \
  --src /tmp/sudachidict-20260723 \
  --dest /tmp/lindera-sudachidict \
  --metadata ./lindera-sudachidict/metadata.json
```

アーカイブには raw レキシコン（small + core + notcore）、連接コスト行列、および位置合わせ済みの `char.def` / `unk.def` が同梱されているため、手動の前処理は不要です。ビルドされた辞書のサイズは約 570MB です。

> [!TIP]
> 同梱バージョンより新しい上流の SudachiDict リリースからビルドするには、
> 上流の raw 配布物から直接ビルドする
> [SudachiDict（カスタムビルド）](../sudachidict.md)を参照してください。

## ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします:

```shell
% lindera build \
  --src ./resources/user_dict/sudachidict_simple_userdic.csv \
  --dest ./resources/user_dict \
  --metadata ./lindera-sudachidict/metadata.json \
  --user
```

ユーザー辞書フォーマットの詳細については、[辞書フォーマット](./dictionary_format.md)を参照してください。

## バイナリへの埋め込み

SudachiDict 辞書をバイナリに直接埋め込むには、以下のようにビルドします:

```shell
cargo build --features=embed-sudachidict
```

これにより、外部辞書ファイルなしで `embedded://sudachidict` を辞書パスとして使用できるようになります。

> [!NOTE]
> 埋め込み辞書はバイナリサイズを約 570MB 増加させます。バイナリサイズが重要な場合は、
> 上記のように辞書をディレクトリにビルドし、パス指定で読み込むことを推奨します。

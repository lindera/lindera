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
  --metadata ./lindera-unidic/metadata.json \
  --context-id-freq ./lindera-unidic/context_id_freq.txt
```

> [!TIP]
> `lindera-unidic/metadata.json` は `connection_id_mapping: true` を設定しているため、ビルダーは
> 連接コスト行列の文脈 ID を使用頻度順に付け替え、連接コスト参照時のキャッシュ局所性を改善します。
> `--context-id-freq` / `-f` に同梱の `context_id_freq.txt` ヒストグラムを渡すことで、この付け替えに
> 実際のコーパス頻度データを与えて ID をランク付けできます。このフラグを省略してもビルドは失敗せず、
> 精度の低いエントリ数ベースのフォールバックに黙って切り替わるだけです。いずれの場合もトークン化の
> 結果には影響しません -- この付け替えはコストを保つ全単射な再ラベル付けであり、ビルド時の最適化のみに
> 関わるものであって正確性には影響しません。

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

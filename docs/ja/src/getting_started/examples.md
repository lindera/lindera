# サンプル

Linderaには、一般的なユースケースを示すいくつかのサンプルプログラムが含まれています。ソースコードはGitHubの [examplesディレクトリ](https://github.com/lindera/lindera/tree/main/lindera/examples) で確認できます。

以下のサンプルはすべて `embed-ipadic` feature を有効にして実行します。この feature はビルド時にIPADIC辞書を自動的にダウンロードしてバイナリに埋め込むため、辞書を手動でダウンロードする必要はありません。

## 利用可能なサンプル

### segment

`Segmenter` API による基本的な形態素分割です。`lindera` クレート単体で動作します。

```shell
cargo run -p lindera --features=embed-ipadic --example=segment
```

### tokenize

外部IPADIC辞書を使用した基本的なトークナイズです。入力テキストを分割し、各トークンの品詞情報を表示します。

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize
```

### tokenize_with_user_dict

ユーザー辞書を使用したトークナイズです。ドメイン固有の用語のために、辞書をカスタムエントリで補完する方法を示します。

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_user_dict
```

### tokenize_with_filters

キャラクターフィルターとトークンフィルターを使用したトークナイズです。Unicode正規化、品詞フィルタリングなどの変換を含むテキスト処理パイプラインを実演します。

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_filters
```

### tokenize_with_config

YAML設定ファイルを使用したトークナイズです。プログラムではなく宣言的にトークナイザーを設定する方法を示します。

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_config
```

# サンプル

Linderaには、一般的なユースケースを示すいくつかのサンプルプログラムが含まれています。ソースコードはGitHubの [examplesディレクトリ](https://github.com/lindera/lindera/tree/main/lindera/examples) で確認できます。

サンプルを実行する前に、[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済みIPADIC辞書をダウンロードし、ローカルディレクトリに展開してください。

## 利用可能なサンプル

### tokenize

外部IPADIC辞書を使用した基本的なトークナイズです。入力テキストを分割し、各トークンの品詞情報を表示します。

```shell
cargo run --example=tokenize
```

### tokenize_with_user_dict

ユーザー辞書を使用したトークナイズです。ドメイン固有の用語のために、辞書をカスタムエントリで補完する方法を示します。

```shell
cargo run --example=tokenize_with_user_dict
```

### tokenize_with_filters

キャラクターフィルターとトークンフィルターを使用したトークナイズです。Unicode正規化、品詞フィルタリングなどの変換を含むテキスト処理パイプラインを実演します。

```shell
cargo run --example=tokenize_with_filters
```

### tokenize_with_config

YAML設定ファイルを使用したトークナイズです。プログラムではなく宣言的にトークナイザーを設定する方法を示します。

```shell
cargo run --example=tokenize_with_config
```

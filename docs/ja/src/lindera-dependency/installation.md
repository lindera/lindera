# インストール

`lindera-dependency` は、Lindera 7.1.0 から crates.io にあります。多くのプログラムは、このクレートに直接は依存しません。Lindera のトークナイザーのトークンフィルター `japanese_dependency` を使います。フィルターは、`lindera` クレート、`lindera` コマンド、各バインディングに入っています。

## トークナイザーの中で使う

フィルターは、`lindera` の `dependency` feature の裏にあります。

```toml
[dependencies]
lindera = { version = "7.1", features = ["dependency"] }
```

`lindera` コマンドとバインディング（Python、Node.js、Ruby、PHP、WebAssembly）は、7.1.0 から、既定のビルドにこの feature を持ちます。インストールしたパッケージに、フィルターが入っています。[トークンフィルター](./token_filter.md)と、それぞれのインストールのページを見てください: [コマンド](../lindera-cli/installation.md)、[Python](../lindera-python/installation.md)、[Node.js](../lindera-nodejs/installation.md)、[Ruby](../lindera-ruby/installation.md)、[PHP](../lindera-php/installation.md)、[WebAssembly](../lindera-wasm/installation.md)。

既定の feature を外してビルドしたコマンドも、フィルターの名前は知っていて、何が足りないかを伝えます。

```text
Error: LinderaError { kind: FeatureDisabled, source: the token filter japanese_dependency needs a build with the feature `dependency` }
```

## クレート

```toml
[dependencies]
lindera-dependency = "7.1"
```

既定のビルドは依存を持たず、`wasm32-unknown-unknown` 向けにビルドできます。Lindera のものでなくても、形態素の列なら受け取れます。[feature フラグ](./architecture.md#feature-フラグ)で、コマンド、学習、埋め込みのモデルを足します。

テキストを解析してラティス形式を書き出す `lindera-dependency` コマンドは、`cli` の裏にあります。

```shell
cargo install lindera-dependency --features cli
```

## モデル

解析には、トークナイザーの辞書に合った[モデルファイル](./model_file.md)が必要です。Lindera は、リリースのたびに、IPADIC 用のモデルを 1 つ公開します（[IPADIC のモデル](./model_file.md#ipadic-のモデル)）。モデルは、辞書と同じく、パッケージにもクレートにも入っていません。手に入れる方法は 3 つあります。

- `lindera download dependency-ipadic` は、コマンドのバージョンのモデルをインストールし、モデルファイルのパスを表示します（[`download`](../lindera-cli/commands.md#係り受け解析のモデルのダウンロード)）。
- [リリース](https://github.com/lindera/lindera/releases)のページにある `lindera-dependency-ipadic-<version>.zip` に、モデルファイル `dependency.model` と、その `NOTICE.txt` が入っています。
- `lindera`、コマンド、バインディングの feature `embed-dependency-ipadic`（`lindera-dependency` では `embed-model-ipadic`）で、ビルドに埋め込みます（[埋め込む](./model_file.md#埋め込む)）。

モデルは MIT ライセンスで、表示を添えてあります（[モデルのライセンス](./model_file.md#モデルのライセンス)）。

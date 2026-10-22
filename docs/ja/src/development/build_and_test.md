# ビルドとテスト

## ビルド

### デフォルトビルド

デフォルトの feature（`mmap`）でワークスペースをビルドします：

```bash
cargo build
```

### 学習機能付きビルド

CRF ベースの辞書学習機能を含めてビルドします：

```bash
cargo build --features train
```

### CLI のみビルド

```bash
cargo build -p lindera-cli
```

CLI ではデフォルトで `train` feature が有効になっています。

## テスト

### 単一テスト

クレート内の特定のテストを実行します（開発時はこちらを推奨）：

```bash
cargo test -p <crate> <test_name>
```

### 学習機能のテスト

```bash
cargo test -p lindera-trainer
```

### クレート単位の全機能テスト

単一クレートの全テストスイートを実行します：

```bash
cargo test -p <crate> --all-features
```

> 注意: CI では `--all-features` は使用されません。各クレートに対してキュレートされた個別の feature の組み合わせでテストを実行します（`.github/workflows/regression.yml` を参照）。Makefile のクレート別パターンターゲット（`make test-<crate>`、`make lint-<crate>`）は CI と同じ feature の組み合わせを適用するため、ローカルでの実行方法としては最も近い代替手段です。CI はさらに、`lindera-segmenter` と `lindera` をすべての辞書を埋め込んだ状態でもう一度テストし（Linux のみ）、ほかの辞書の feature でだけ有効になるテストも実行します。ローカルで同じことをするには、`cargo test -p lindera --features embed-ipadic,embed-ipadic-neologd,embed-unidic,embed-sudachidict,embed-ko-dic,embed-cc-cedict,embed-jieba` を実行します（`lindera-segmenter` も同様）。

### ワークスペース全体のテスト

```bash
cargo test
```

## 品質チェック

### フォーマットチェック

コードのフォーマットがプロジェクトのスタイルに一致しているか確認します：

```bash
cargo fmt --all -- --check
```

フォーマットを自動修正するには：

```bash
cargo fmt --all
```

> 注意: `cargo fmt` が整形するのは Rust のコードだけです。言語バインディングでは、`make format-lindera-<binding>` がバインディングの Python・JavaScript・Ruby・PHP のコードも整形します。使うツールと版は、`make lint-lindera-<binding>` が検査に使うものと同じです（下記を参照）。`make lint-scripts` が検査する、クレートの外のファイルは、`make format-scripts` が同じように整形します。

### リント

Clippy を警告をエラーとして扱うモードで実行します：

```bash
cargo clippy -- -D warnings
```

> 注意: CI は各クレートに対して、そのクレートのテストと同じ feature で `cargo clippy --all-targets -- -D warnings` を実行します（`.github/workflows/test-crate.yml` と、`.github/workflows/regression.yml` の言語バインディングのジョブを参照）。そのため、テストのコードも lint されます。`make lint-<crate>` は同じ clippy のコマンドを実行します。PR を開く前にローカルで（例えば `make lint` 経由で）clippy を実行してください。
>
> 言語バインディングでは、`make lint-lindera-<binding>` が続けて、そのバインディングの CI ジョブと同じく言語側の linter を実行します。Python は ruff、Node.js と WASM は Prettier、Ruby は RuboCop、PHP は PHP CS Fixer です。これらの版は固定してあるので（ruff と Prettier は `Makefile`、RuboCop は `lindera-ruby/Gemfile`、PHP CS Fixer は `composer.json`）、ツールの新しいリリースが、変更していないコードで CI を失敗させることはありません。ターゲットは固定した版を自分でインストールするので、その言語のツールチェーン（Python 3、npx の使える Node.js、Bundler、Composer）とネットワークへの接続が必要です。
>
> `make lint-scripts` は、どのバインディングにも属さない手書きのファイルを、CI の `Lint scripts` ジョブと同じく lint します。対象は、バインディングが共有する `scripts/benchmarks/` のベンチマーク一式と、docs が読み込む JavaScript（`docs/mermaid-init.js`）です。同じ固定版のツールを使い、各ファイルにはその言語のバインディングの規則を当てるので、4 つの言語のツールチェーンがすべて必要です。`make lint` は、クレートの後にこれも実行します。

## ドキュメント

### API ドキュメント

Rust の API ドキュメントを生成して開きます：

```bash
cargo doc --no-deps --open
```

各クレートの API ドキュメントに rustdoc の警告（壊れた intra-doc link や、公開項目から private 項目へのリンクなど）がないかを確認します：

```bash
make check-docs
```

> 注意: CI は `make check-docs` を実行し、rustdoc の警告が 1 つでもあれば失敗します。言語バインディング以外のクレートを、すべての feature を有効にして文書化します。ライブラリは 2 回文書化し、1 回目は docs.rs が公開するのと同じく公開項目だけ、2 回目は private 項目も含めます。private 項目の doc コメントを検査するのは 2 回目だけです。続けて、CLI のコードがある `lindera-cli` のバイナリを private 項目も含めて文書化します。ダミーの辞書（`DOCS_RS=1`）を使い、専用の target ディレクトリ `target/check-docs` でビルドするので、辞書をダウンロードせず、`target/` にも影響しません。

言語バインディングは、ビルドにそれぞれの言語のツールチェーンが必要なため、1 つずつ検査します：

```bash
make check-docs-lindera-python   # -nodejs、-ruby、-php、-wasm も同様
```

> 注意: 各バインディングの CI ジョブがそれぞれのターゲットを実行し、rustdoc の警告が 1 つでもあれば失敗します。ターゲットはバインディングを 2 回文書化します。1 回目は公開項目だけ、2 回目は private 項目も含めます。feature はデフォルトのままです。項目の有無を切り替える feature はすべてデフォルトで有効なので、これで足ります。辞書はビルドしません。`lindera-ruby` は `make test-lindera-ruby` と同じ RbConfig のラッパーを通して実行し、`lindera-wasm` は `wasm32-unknown-unknown` 向けに文書化します。

### mdBook ドキュメント

ユーザー向けドキュメントをビルドします：

```bash
mdbook build docs
```

`http://localhost:3000` でローカルプレビュー：

```bash
mdbook serve docs
```

### Markdown リント

ドキュメントの Markdown スタイルの問題をチェックします：

```bash
markdownlint-cli2 "docs/src/**/*.md"
```

ルールはリポジトリルートの `.markdownlint.json` で設定されています。

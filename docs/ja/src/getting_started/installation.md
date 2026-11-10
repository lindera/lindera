# インストール

## 動作要件

Rust **1.89** 以降。ワークスペースが `rust-version` として宣言しているため、
古いツールチェインではコンパイルエラーではなく cargo のエラーとして報告されます。

## 依存関係の追加

Cargo.tomlに以下を追加してください：

```toml
[dependencies]
lindera = "7"
```

> [!NOTE]
> この 1 つの依存だけで、分析チェーン（character filter・token filter・`Tokenizer`）も
> `lindera::analysis` として利用できます。セグメンターだけが必要な場合は
> `default-features = false` を指定してください。
> v6 からアップグレードする場合は[v6 から v7 への移行](../migration_v6_to_v7.md)を参照してください。

## 辞書のセットアップ

Linderaの実行にはビルド済み辞書が必要です。[GitHub Releases](https://github.com/lindera/lindera/releases) から辞書をダウンロードし、読み込み時にそのパスを指定してください：

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```rust
> // Cargo.toml: lindera = { version = "7", features = ["embed-ipadic"] }
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> `embed-*` のビルドは、辞書クレートのために Lindera が GitHub で公開している辞書をダウンロードし、SHA-256 を確かめて埋め込みます。辞書をソースからビルドしなくなったので、ビルドには `lindera.dev` ではなく `github.com`（ファイルは `release-assets.githubusercontent.com` から配られます）への接続が要ります。`LINDERA_BUILD_DICTIONARY_CACHE_DIR` を設定すれば、2 回目からのビルドはダウンロードした辞書を使い、ネットワークに接続しません。build script がソースからビルドする場合は、[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](./environment_variables.md#lindera_build_dictionary_from_source) を、ミラーからダウンロードするには [`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`](./environment_variables.md#lindera_build_dictionary_published_base_url) を参照してください。
>
> 埋め込めるのは同梱の辞書だけです。自分の辞書は `lindera build` でビルドし、実行時にそのディレクトリから読み込んでください。
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## 環境変数

辞書クレートの build script は `LINDERA_BUILD_DICTIONARY_CACHE_DIR`・`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`・`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`・`LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL`・`LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`・`DOCS_RS` を、`TokenizerBuilder::new()` は `LINDERA_CONFIG_FILE` を読みます。これらを含め、Lindera が読む環境変数は[環境変数](./environment_variables.md)を参照してください。

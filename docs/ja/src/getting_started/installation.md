# インストール

Cargo.tomlに以下を追加してください：

```toml
[dependencies]
lindera = "5.0"
```

> [!NOTE]
> v5.0.0 は次期リリース予定であり、まだ crates.io には公開されていません。現時点での公開バージョンは
> `4.0.1` です。本ガイドは `main` ブランチに既に存在する v5.0.0 の API を説明しています。詳細は
> [v4からv5への移行](../migration_v4_to_v5.md)を参照してください。

## 辞書のセットアップ

Linderaの実行にはビルド済み辞書が必要です。[GitHub Releases](https://github.com/lindera/lindera/releases) から辞書をダウンロードし、読み込み時にそのパスを指定してください：

```rust
let dictionary = load_dictionary("/path/to/ipadic")?;
```

> [!TIP]
> 辞書をバイナリに直接埋め込みたい場合（上級者向け）は、対応する `embed-*` feature フラグを有効にしてビルドし、`embedded://` スキームでロードしてください：
>
> ```rust
> // Cargo.toml: lindera = { version = "5.0", features = ["embed-ipadic"] }
> let dictionary = load_dictionary("embedded://ipadic")?;
> ```
>
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## 環境変数

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

`LINDERA_BUILD_DICTIONARY_CACHE_DIR` 環境変数は、埋め込み辞書ビルドパイプラインのビルド時キャッシュディレクトリを指定します。辞書クレートの build script のみが読み取り、実行時の動作には影響しません。

設定すると、各ビルドは `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>/`（`<version>` は辞書クレートのバージョン）配下に 2 種類のファイルを保存します：

- ダウンロードした配布アーカイブ（MD5 で検証。無効なファイルは自動的に再ダウンロード）
- クレートに埋め込まれるビルド済みバイナリ辞書

これにより以下のメリットがあります：

- **オフラインビルド**: 一度キャッシュされれば、以降のビルドにネットワークアクセスは不要です
- **ビルドの高速化**: 有効なキャッシュがあればダウンロードと辞書ビルドがスキップされます
- **再現可能なビルド**: ビルド間での辞書バージョンの一貫性を保ちます

使用方法：

```shell
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
cargo build --features=embed-ipadic
```

注意点：

- このディレクトリは自動管理されており、削除しても安全です（必要に応じて再ダウンロード・再ビルドされます）
- バージョンごとのサブディレクトリはアップグレードのたびに蓄積され、自動削除されません。古いものは自由に削除できます
- この変数を設定すると、`embed-*` feature が無効でも辞書クレートはダウンロードとビルドを実行します（キャッシュの事前準備に便利です）

> **非推奨:** 旧名 `LINDERA_DICTIONARIES_PATH` はフォールバックとして引き続き動作しますが（両方設定時は新名が優先）、v6.0.0 で削除されます。

### LINDERA_CONFIG_PATH

`LINDERA_CONFIG_PATH` 環境変数は、トークナイザーの設定ファイル（YAML形式）へのパスを指定します。これにより、Rustコードを変更せずにトークナイザーの動作を設定できます。

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

設定フォーマットの詳細は [設定](../lindera-analysis/configuration.md) セクションを参照してください。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rsでドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Linderaは実際の辞書データをダウンロードする代わりにダミーの辞書ファイルを作成します。これにより、ネットワークアクセスや大容量ファイルのダウンロードなしでドキュメントをビルドできます。

これは主にdocs.rs内部で使用されるものであり、通常ユーザーが設定する必要はありません。

### LINDERA_WORKDIR

`LINDERA_WORKDIR` 環境変数は、ビルドプロセス中に lindera-dictionary クレートによって自動的に設定されます。これはビルドされた辞書データファイルを含むディレクトリを指し、辞書クレートがデータファイルの場所を特定するために内部で使用されます。

この変数は自動的に設定されるため、ユーザーが変更する必要はありません。

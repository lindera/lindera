# インストール

Cargo.tomlに以下を追加してください：

```toml
[dependencies]
lindera = { version = "2.1.1", features = ["embed-ipadic"] }
```

## 環境変数

### LINDERA_DICTIONARIES_PATH

`LINDERA_DICTIONARIES_PATH` 環境変数は、辞書ソースファイルをキャッシュするディレクトリを指定します。これにより以下のメリットがあります：

- **オフラインビルド**: 一度ダウンロードすれば、将来のビルドのために辞書ソースファイルが保存されます
- **ビルドの高速化**: 有効なキャッシュファイルが存在する場合、次回のビルドではダウンロードがスキップされます
- **再現可能なビルド**: ビルド間での辞書バージョンの一貫性を保ちます

使用方法：

```shell
export LINDERA_DICTIONARIES_PATH=/path/to/dicts
cargo build --features=ipadic
```

設定された場合、辞書ソースファイルは `$LINDERA_DICTIONARIES_PATH/<version>/` （`<version>` は lindera-dictionary クレートのバージョン）に保存されます。キャッシュはMD5チェックサムを使用してファイルを検証し、無効なファイルは自動的に再ダウンロードされます。

> [!NOTE]
> `LINDERA_CACHE` は非推奨ですが、後方互換性のために引き続きサポートされています。`LINDERA_DICTIONARIES_PATH` が設定されていない場合に使用されます。

### LINDERA_CONFIG_PATH

`LINDERA_CONFIG_PATH` 環境変数は、トークナイザーの設定ファイル（YAML形式）へのパスを指定します。これにより、Rustコードを変更せずにトークナイザーの動作を設定できます。

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

設定フォーマットの詳細は [設定](./configuration.md) セクションを参照してください。

> [!NOTE]
> `LINDERA_CONFIG_PATH` は非推奨ですが、後方互換性のために引き続きサポートされています。`LINDERA_CONFIG_PATH` が設定されていない場合に使用されます。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rsでドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Linderaは実際の辞書データをダウンロードする代わりにダミーの辞書ファイルを作成します。これにより、ネットワークアクセスや大容量ファイルのダウンロードなしでドキュメントをビルドできます。

これは主にdocs.rs内部で使用されるものであり、通常ユーザーが設定する必要はありません。

### LINDERA_WORKDIR

`LINDERA_WORKDIR` 環境変数は、ビルドプロセス中に lindera-dictionary クレートによって自動的に設定されます。これはビルドされた辞書データファイルを含むディレクトリを指し、辞書クレートがデータファイルの場所を特定するために内部で使用されます。

この変数は自動的に設定されるため、ユーザーが変更する必要はありません。

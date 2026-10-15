# インストール

## 動作要件

Rust **1.88** 以降。ワークスペースが `rust-version` として宣言しているため、
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
> 詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

## 環境変数

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

`LINDERA_BUILD_DICTIONARY_CACHE_DIR` 環境変数は、埋め込み辞書ビルドパイプラインのビルド時キャッシュディレクトリを指定します。辞書クレートの build script のみが読み取り、実行時の動作には影響しません。

設定すると、各ビルドは `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>-fmt<format>/`（`<version>` は辞書クレートのバージョン、`<format>` は辞書フォーマットバージョン）配下に 2 種類のファイルを保存します：

- ダウンロードした配布アーカイブ（MD5 で検証。無効なファイルは自動的に再ダウンロード）
- クレートに埋め込まれるビルド済みバイナリ辞書と、その辞書を何から作ったかを記録する `build_inputs.json`

フォーマットバージョンをパスに含めているのは、オンディスクレイアウトが異なるビルドが書いたキャッシュを「古いまま再利用」ではなく「キャッシュミス」にするためです。キャッシュした辞書は、次のすべてが今回のビルドと一致するときだけ再利用し、そうでなければ作り直します。

- 辞書フォーマットバージョンと、クレートの `metadata.json` の内容
- 辞書を作る `lindera-dictionary` クレートのバージョンとソースのダイジェスト。辞書ビルダーを変更すると、コミット前の変更でも辞書を作り直します
- ソースアーカイブ（ファイル名と MD5）と、ビルドがその中で読むディレクトリ
- `connection_id_mapping` が有効なとき、ビルドが読む文脈 ID の頻度表（`context_id_freq.txt`、または `LINDERA_CTX_FREQ_FILE` が指すファイル）

中断されたビルドが残した辞書や、`build_inputs.json` より前のリリースがキャッシュした辞書は作り直します。ビルダーが依存するクレートのバージョンは比較しません。フォーマットバージョンを変えずに `lindera-dictionary` の依存クレートを更新したときは、ビルドした辞書が変わりうるならキャッシュを削除してください。古いフォーマットバージョンのディレクトリは自動削除されないため、キャッシュが肥大化する場合は手動で削除してください。

`cargo build -vv` を実行すると、辞書クレートごとに、キャッシュした辞書を使ったかどうかとその理由が `lindera dictionary cache:` で始まる行に表示されます。

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
- この変数を設定すると、`embed-*` feature が無効でも辞書クレートはダウンロードとビルドを実行します（キャッシュの事前準備に便利です）。このビルドは辞書を埋め込まないので、キャッシュにすでにある辞書は確かめずにそのまま残します。`embed-*` feature を有効にしたビルドが確かめ、必要なら作り直します
- キャッシュディレクトリは checkout ごとに 1 つ使ってください。辞書ビルダーのコードが異なる 2 つの checkout が同じディレクトリを共有すると、互いの辞書を作り直し合います。さらに、build script が再実行されなかったクレートは、もう一方の checkout が作った辞書を埋め込むことがあります。このリポジトリでは相対パスがワークスペースのルートを基準に解決されるので、checkout ごとに別のキャッシュになります
- `DOCS_RS` を設定したビルドは、この変数を無視します（後述の `DOCS_RS` を参照）

### LINDERA_CONFIG_PATH

`LINDERA_CONFIG_PATH` 環境変数は、トークナイザーの設定ファイル（YAML形式）へのパスを指定します。これにより、Rustコードを変更せずにトークナイザーの動作を設定できます。

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

設定フォーマットの詳細は [設定](../lindera-analysis/configuration.md) セクションを参照してください。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rsでドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Linderaは実際の辞書データをダウンロードする代わりにダミーの辞書ファイルを作成します。これにより、ネットワークアクセスや大容量ファイルのダウンロードなしでドキュメントをビルドできます。

このビルドは `LINDERA_BUILD_DICTIONARY_CACHE_DIR` を無視します。ダミーの辞書は build script の出力ディレクトリに作られ、キャッシュには入らないので、後で `DOCS_RS` なしにビルドしても埋め込まれません。

これは主にdocs.rs内部で使用されるものであり、通常ユーザーが設定する必要はありません。

### LINDERA_WORKDIR

`LINDERA_WORKDIR` 環境変数は、ビルドプロセス中に lindera-dictionary クレートによって自動的に設定されます。これはビルドされた辞書データファイルを含むディレクトリを指し、辞書クレートがデータファイルの場所を特定するために内部で使用されます。

この変数は自動的に設定されるため、ユーザーが変更する必要はありません。

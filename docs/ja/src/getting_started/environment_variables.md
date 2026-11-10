# 環境変数

Lindera は次の環境変数を読みます。どれも設定しなくても動きます。

| 変数 | 読むもの | 値 | 設定しないとき |
| --- | --- | --- | --- |
| [`LINDERA_BUILD_DICTIONARY_CACHE_DIR`](#lindera_build_dictionary_cache_dir) | 辞書クレートの build script | ディレクトリ | ビルドごとに、その出力ディレクトリで辞書をダウンロードしてビルドする |
| [`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source) | 辞書クレートの build script | `1` か `true` | 辞書クレートは固定した公開済みの辞書をダウンロードし、それを使えないときだけソースからビルドする |
| [`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`](#lindera_build_dictionary_published_base_url) | 辞書クレートの build script | `http` か `https` の URL | 公開済みの辞書を GitHub のリリースからダウンロードする |
| [`LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL`](#lindera_build_dictionary_source_base_url) | 辞書クレートの build script | `http` か `https` の URL | ソースのアーカイブを `https://lindera.dev` からダウンロードする |
| [`LINDERA_BUILD_CONTEXT_ID_FREQ_FILE`](#lindera_build_context_id_freq_file) | 辞書クレートの build script | ファイル | 辞書クレートに同梱の頻度表（あれば） |
| [`DOCS_RS`](#docs_rs) | 辞書クレートの build script | docs.rs が設定する | 本物の辞書をビルドする |
| [`LINDERA_CONFIG_FILE`](#lindera_config_file) | `TokenizerBuilder::new()`（`lindera tokenize` とバインディングも） | ファイル | 空の設定 |
| [`LINDERA_DICTIONARIES_DIR`](#lindera_dictionaries_dir) | CLI | ディレクトリ | OS のローカルデータディレクトリの `lindera/dictionaries` |
| [`RUST_LOG`](#rust_log) | CLI | ログのフィルター | 警告とエラー |
| [`RUST_LOG_STYLE`](#rust_log) | CLI | 色の設定（`never` など） | 標準エラー出力が端末なら色を付ける |
| [`LINDERA_FEATURES`](#lindera_features) | Ruby の gem のビルド | feature のカンマ区切りリスト | デフォルトの feature |

このうち 3 つには、v8.0.0 まで非推奨の警告つきで使える旧名があります。`LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` の旧名 `LINDERA_CTX_FREQ_FILE`、`LINDERA_CONFIG_FILE` の旧名 `LINDERA_CONFIG_PATH`、そして `LINDERA_DICTIONARIES_DIR=<dir>/dictionaries` として読む `LINDERA_DATA_DIR` です。[v6 から v7 への移行](../migration_v6_to_v7.md)を参照してください。

## ビルド時の変数

`lindera-ipadic` などの辞書クレートの build script は、辞書をビルドするとき（たとえば `lindera` の `embed-*` feature を有効にしたとき）にこれらの変数を読みます。実行時の動作には影響しません。

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

`LINDERA_BUILD_DICTIONARY_CACHE_DIR` 環境変数は、埋め込み辞書ビルドパイプラインのビルド時キャッシュディレクトリを指定します。辞書クレートの build script のみが読み取り、実行時の動作には影響しません。

設定すると、各ビルドは `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>-fmt<format>/`（`<version>` は辞書クレートのバージョン、`<format>` は辞書フォーマットバージョン）配下に 2 種類のファイルを保存します：

- ダウンロードした配布アーカイブ（クレートの `build.rs` が固定した SHA-256 で検証。無効なファイルは自動的に再ダウンロード）
- ビルド済みバイナリ辞書と、その辞書を何から作ったかを記録する `build_inputs.json`

フォーマットバージョンをパスに含めているのは、オンディスクレイアウトが異なるビルドが書いたキャッシュを「古いまま再利用」ではなく「キャッシュミス」にするためです。キャッシュした辞書は、次のすべてが今回のビルドと一致するときだけ再利用し、そうでなければ作り直します。

- 辞書フォーマットバージョンと、クレートの `metadata.json` の内容
- 辞書を作る `lindera-dictionary` クレートのバージョンとソースのダイジェスト。辞書ビルダーを変更すると、コミット前の変更でも辞書を作り直します
- ソースアーカイブ（ファイル名と SHA-256）と、ビルドがその中で読むディレクトリ
- `connection_id_mapping` が有効なとき、ビルドが読む文脈 ID の頻度表（`context_id_freq.txt`、または `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` が指すファイル）

ビルドせずにダウンロードした公開済みの辞書（[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source) を参照）も同じ場所にキャッシュし、`build_inputs.json` には上の入力に加えて、その辞書の鍵と SHA-256 を記録します。同じ公開済みの辞書をダウンロードするはずの次のビルドは、キャッシュした辞書を使います。公開済みの辞書とソースからのビルドを切り替えたビルドは、作り直します。

中断されたビルドが残した辞書や、`build_inputs.json` より前のリリースがキャッシュした辞書は作り直します。ビルダーが依存するクレートのバージョンは比較しません。フォーマットバージョンを変えずに `lindera-dictionary` の依存クレートを更新したときは、ビルドした辞書が変わりうるならキャッシュを削除してください。古いフォーマットバージョンのディレクトリは自動削除されないため、キャッシュが肥大化する場合は手動で削除してください。

ビルドは、キャッシュしたファイルそのものは埋め込みません。辞書の 9 つのファイルを build script の出力ディレクトリ（`OUT_DIR`）にハードリンクし（キャッシュがターゲットディレクトリと別のファイルシステムにあるときなど、リンクできないファイルはコピーし）、そこから埋め込みます。そのため、入力の異なる別のビルドがキャッシュした辞書を置き換えても、キャッシュを削除しても、クレートが埋め込んだ辞書は変わりません。どの checkout やターゲットディレクトリのビルドも、1 つのディレクトリを共有できます。

`cargo build -vv` を実行すると、辞書クレートごとに、キャッシュした辞書を使ったかどうかとその理由、辞書をリンクしたかコピーしたかが `lindera dictionary cache:` で始まる行に表示されます。

これにより以下のメリットがあります：

- **オフラインビルド**: 一度キャッシュされれば、以降のビルドにネットワークアクセスは不要です
- **ビルドの高速化**: 有効なキャッシュがあればダウンロードと辞書ビルドがスキップされます
- **再現可能なビルド**: ビルド間での辞書バージョンの一貫性を保ちます

使用方法：

```shell
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
cargo build --features=embed-ipadic
```

すべてのターゲットディレクトリと worktree で 1 つのディレクトリを共有するには、絶対パスを一度だけ設定します。上のようにシェルのプロファイルに書くか、cargo が build script に渡す `~/.cargo/config.toml` に書きます：

```toml
[env]
LINDERA_BUILD_DICTIONARY_CACHE_DIR = "/path/to/cache"
```

`[env]` の値を変えると、変数を変えたときと同じく build script が再実行されます。シェルで設定した変数は `[env]` より優先されます。

注意点：

- このディレクトリは自動管理されており、削除しても安全です。ビルド済みのクレートはリンクまたはコピーした辞書を持ち続け、辞書が必要な次のビルドが再ダウンロード・再ビルドします
- バージョンごとのサブディレクトリはアップグレードのたびに蓄積され、自動削除されません。古いものは自由に削除できます
- この変数を設定すると、`embed-*` feature が無効でも辞書クレートはダウンロードとビルドを実行します（キャッシュの事前準備に便利です）。このビルドは辞書を埋め込まないので、キャッシュにすでにある辞書は確かめずにそのまま残します。`embed-*` feature を有効にしたビルドが確かめ、必要なら作り直します
- 相対パスはワークスペースのルートを基準に解決されるので、このリポジトリでは checkout ごとに別のキャッシュになります。共有するには絶対パスを設定してください
- ビルダーが異なる checkout や、入力が異なるビルド（`LINDERA_BUILD_DICTIONARY_FROM_SOURCE` など）は、build script が実行されるたびに互いのキャッシュした辞書を置き換えます。そのたびにダウンロードやビルドがかかりますが、ビルド済みのクレートが埋め込んだ辞書は変わりません
- CI の実行をまたいでキャッシュを残す（たとえば `actions/cache` で）ときは、`lindera-dictionary` と辞書クレートに加えて `Cargo.lock` でも変わるキーにしてください。ビルダーが依存するクレートのバージョンは比較しないためです。このリポジトリの CI は、こうしたキーで辞書ごとに 1 つのエントリを残します（`.github/actions/dictionary-cache`）
- このディレクトリを共有するビルド（たとえば `cargo build` と、別のターゲットディレクトリを使う rust-analyzer）は、辞書を 1 つずつ書き込みます。同じ辞書を別のビルドが書き込んでいれば、それが終わるのを待ち、そのビルドが残した辞書を使います（`cargo build -vv` に `lindera dictionary cache: waiting for another build` と表示されます）。ロックは辞書の隣にある空の `<辞書>.lock` ファイルで、ビルドが辞書を書き込む間だけ保持します。ロックが効くのは 1 台のマシンの中のビルド同士で、ネットワークファイルシステムでは、そのファイルシステムがファイルロックに対応する範囲に限られます。ファイルをロックできないときは、ビルドは警告を出し、7.0.0 より前と同じくロックなしで続けるので、そうしたビルド同士は互いのファイルを消すことがあります
- `DOCS_RS` を設定したビルドは、この変数を無視します（後述の `DOCS_RS` を参照）

### LINDERA_BUILD_DICTIONARY_FROM_SOURCE

公式の 7 つの辞書は、入力が変わるたびに一度だけビルドし、辞書と版ごとに 1 つの GitHub のリリースとして公開します。公開したリリースは置き換えません。各辞書クレートは、`metadata.json` の隣の `prebuilt.json` で、公開済みの辞書を URL と SHA-256 で固定しています。build script は、ソースのアーカイブを取って辞書をビルドする代わりに、そのアーカイブをダウンロードし、SHA-256 を確かめて埋め込みます。アーカイブの中身はソースからビルドしたものと同じで、違うのは `metadata.json` の `lindera_version` だけです。これは、その辞書をビルドした Lindera の版を表します。公開の手順は[辞書のリリース](../development/dictionary_release.md)にあります。

ダウンロードの失敗、SHA-256 の違うアーカイブ、ファイルが欠けているかクレートの `metadata.json` に合わないアーカイブは、何が起きたかを示すエラーでビルドを止めます。確かめられなかったものは埋め込まず、ソースからのビルドにも切り替えません。

build script は、次の場合に v6 までと同じくソースから辞書をビルドします。

- `LINDERA_BUILD_DICTIONARY_FROM_SOURCE` が `1` か `true` のとき。`0`、`false`、空の値は、build script の判断に任せます。ほかの値は、警告を出したうえで `1` とみなします。
- `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` が頻度表を指すとき。辞書が公開済みのものと変わるためです。
- 公開済みの辞書の辞書フォーマットバージョンが、このビルドの読むものと違うとき。開発中のブランチで、フォーマットを変えてから新しい辞書を公開するまでの間がこれにあたります。
- `prebuilt.json` がまだ SHA-256 を固定していないとき。新しい辞書が公開を待っている間です。

はじめの 3 つの場合は、build script が理由を警告で示します。たとえば次のとおりです。

```text
warning: lindera-ipadic@7.0.0: building the ipadic dictionary from source, because LINDERA_BUILD_DICTIONARY_FROM_SOURCE is set
```

Cargo がこの警告を表示するのは、このリポジトリの checkout のように、パスから辞書クレートをビルドするときです。crates.io の辞書クレートでは、ビルドが失敗したときと `cargo build -vv` を付けたときだけ表示します。変数や `prebuilt.json` を変えると、build script が再実行されます。`DOCS_RS` を設定したビルドは、どちらもせずにダミーの辞書を使います（後述の `DOCS_RS` を参照）。

```shell
LINDERA_BUILD_DICTIONARY_FROM_SOURCE=1 cargo build --features=embed-ipadic
```

### LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL

`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL` と [`LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL`](#lindera_build_dictionary_source_base_url) は、build script のダウンロード先をミラーに向けます。Artifactory のようなプロキシのリポジトリを通すビルドなど、Lindera が公開しているホストに届いてはいけないビルドのための変数です。それぞれが 1 つのホストを置き換えます。

| 変数 | 置き換えるもの | ビルドがダウンロードするもの |
| --- | --- | --- |
| `LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL` | `https://github.com/lindera/lindera/releases/download`（クレートの `prebuilt.json` の `base_url`） | `<base>/<path>`（`path` は `prebuilt.json` のもの） |
| `LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL` | `https://lindera.dev`（クレートの `build.rs` がソースのアーカイブを取るホスト） | `<base>/<file_name>`（`file_name` はアーカイブのファイル名） |

IPADIC の場合、変数を `https://mirror.example/lindera-releases` と `https://mirror.example/lindera-dev` にすると、ビルドは次のどちらかをダウンロードします。

```text
https://mirror.example/lindera-releases/dictionary-ipadic-2.7.0-20250920-fmt3-r1/ipadic-2.7.0-20250920-fmt3-r1.tar.gz
https://mirror.example/lindera-dev/mecab-ipadic-2.7.0-20250920.tar.gz
```

つまり、ミラーには元のホストと同じパスでファイルを置きます。Artifactory なら、`https://github.com/lindera/lindera/releases/download` をプロキシする remote repository が公開済みの辞書を、`https://lindera.dev` をプロキシするものがソースのアーカイブを配ります。ビルドは、既定では公開済みの辞書をダウンロードし、ソースからビルドするときだけソースのアーカイブをダウンロードします（[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source) を参照）。ソースからのビルドは `LINDERA_BUILD_DICTIONARY_FROM_SOURCE` がなくても起こるので、元のホストに届いてはいけないビルドでは、両方の変数を設定します。

```shell
export LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL=https://mirror.example/lindera-releases
export LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL=https://mirror.example/lindera-dev
cargo build --features=embed-ipadic
```

- ミラーはホストを置き換えます。ビルドはミラーだけからダウンロードし、ミラーが失敗しても元のホストには行きません。
- ビルドは、元のホストから取るときと同じく、ダウンロードしたものを辞書クレートが固定した SHA-256 と照合します。別のファイルを返すミラーは、ビルドを止めます。
- 値は `http` か `https` の URL です。末尾の `/` は無視します。ビルドは認証情報を送らないので、ミラーは認証なしでファイルを配る必要があります。ユーザー名やパスワードを含む URL はエラーです。問い合わせ文字列やフラグメントを含む URL も、後ろにパスを付けられないのでエラーです。空の値はミラーなしとみなします。
- `LINDERA_BUILD_DICTIONARY_CACHE_DIR` のキャッシュは、辞書とソースのアーカイブを、取ってきたホストではなく SHA-256 で記録します。そのため、ミラーを使うビルドは、ミラーを使わないビルドがキャッシュしたものを使い、その逆も同じです。
- どちらの変数を変えても、build script は再実行されません。どのホストから取っても辞書は同じだからです。元のホストに届かなかったなどで失敗した build script は、次のビルドで改めて実行されます。
- この変数を読むのは build script だけです。`lindera download` は、実行時に GitHub から辞書をダウンロードします。

### LINDERA_BUILD_DICTIONARY_SOURCE_BASE_URL

辞書のソースのアーカイブのミラーです。ビルドは `https://lindera.dev/<file_name>` の代わりに `<base>/<file_name>` をダウンロードします。2 つの変数の説明は [`LINDERA_BUILD_DICTIONARY_PUBLISHED_BASE_URL`](#lindera_build_dictionary_published_base_url) にあります。

### LINDERA_BUILD_CONTEXT_ID_FREQ_FILE

`LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` 環境変数は、辞書クレートの build script が、クレートに同梱の `context_id_freq.txt` の代わりに読む文脈 ID の頻度表を指定します。効くのは `metadata.json` で `connection_id_mapping` を有効にした辞書（同梱の辞書では UniDic と ko-dic）だけで、実行時の動作には影響しません。頻度表は、よく使う文脈 ID が近くに並ぶように連接コスト行列の文脈 ID を並べ替えるためのもので、トークナイズの速さは変わりますが、結果は変わりません。`lindera` クレートの例 `ctxfreq_dump` で、コーパスから頻度表を作れます。

```shell
export LINDERA_BUILD_CONTEXT_ID_FREQ_FILE=/path/to/context_id_freq.txt
cargo build --features=embed-unidic
```

この変数を読むのは build script だけです。`lindera build` は `--context-id-freq` オプションで頻度表を受け取り、バインディングの辞書をビルドする関数は省略可能な引数 `context_id_freq`（Node.js では `contextIdFreq`）で受け取ります。変数や、変数が指すファイルを変えると build script が再実行され、`LINDERA_BUILD_DICTIONARY_CACHE_DIR` のキャッシュは頻度表の内容を比べます。この変数を設定すると、build script は公開済みの辞書をダウンロードせずに、ソースから辞書をビルドします（[`LINDERA_BUILD_DICTIONARY_FROM_SOURCE`](#lindera_build_dictionary_from_source) を参照）。

v6 までの名前 `LINDERA_CTX_FREQ_FILE` も v8.0.0 までは使えますが、非推奨の警告が出ます：

```text
warning: lindera-ko-dic@7.0.0: LINDERA_CTX_FREQ_FILE is deprecated and will be removed in v8.0.0; set LINDERA_BUILD_CONTEXT_ID_FREQ_FILE instead
```

両方を設定すると `LINDERA_BUILD_CONTEXT_ID_FREQ_FILE` が使われ、`LINDERA_CTX_FREQ_FILE` は無視されると警告します。Cargo がこれらの警告を表示するのは、このリポジトリの checkout のように、パスで指定した辞書クレートをビルドするときです。crates.io から取得した辞書クレートでは、ビルドが失敗したときか `cargo build -vv` のときだけ表示します。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rsでドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Linderaは実際の辞書データをダウンロードする代わりにダミーの辞書ファイルを作成します。これにより、ネットワークアクセスや大容量ファイルのダウンロードなしでドキュメントをビルドできます。

このビルドは `LINDERA_BUILD_DICTIONARY_CACHE_DIR` を無視します。ダミーの辞書は build script の出力ディレクトリに作られ、キャッシュには入らないので、後で `DOCS_RS` なしにビルドしても埋め込まれません。

これは主にdocs.rs内部で使用されるものであり、通常ユーザーが設定する必要はありません。

## 実行時の変数

### LINDERA_CONFIG_FILE

`LINDERA_CONFIG_FILE` 環境変数は、`TokenizerBuilder::new()` が読むトークナイザーの設定ファイル（YAML形式）を指定します。これにより、Rustコードを変更せずにトークナイザーの動作を設定できます。

```shell
export LINDERA_CONFIG_FILE=./resources/config/lindera.yml
```

v6 までの名前 `LINDERA_CONFIG_PATH` も v8.0.0 までは使えますが、標準エラー出力に非推奨の警告が出ます。両方を設定すると `LINDERA_CONFIG_FILE` が使われます。

設定フォーマットの詳細は [設定](../lindera-analysis/configuration.md) セクションを参照してください。

## CLI の変数

`lindera` コマンドがこれらの変数を読みます。コマンドについては Lindera CLI の[コマンド](../lindera-cli/commands.md)を参照してください。

### LINDERA_DICTIONARIES_DIR

`LINDERA_DICTIONARIES_DIR` 環境変数は辞書ディレクトリを指定します。`lindera download <辞書名>` は辞書を `<辞書ディレクトリ>/<辞書名>/` にインストールし、`lindera list` はそこにある辞書を表示し、`--dict` や `--system-dict` に辞書名を渡したときのエラーはそこのパスを示します。指定しない場合は、OS のローカルデータディレクトリの `lindera/dictionaries` になります：

| OS | デフォルトの場所 |
| --- | --- |
| Linux | `~/.local/share/lindera/dictionaries` |
| macOS | `~/Library/Application Support/lindera/dictionaries` |
| Windows | `%LOCALAPPDATA%\lindera\dictionaries` |

たとえば `LINDERA_DICTIONARIES_DIR=/var/lindera/dict` なら、`lindera download ipadic` は `/var/lindera/dict/ipadic/` にインストールします。`lindera list` も同じディレクトリの辞書を表示します。

v7.0.0 より前の CLI の変数 `LINDERA_DATA_DIR` は、v8.0.0 まで標準エラー出力への警告つきで使えます。値 `<dir>` は `LINDERA_DICTIONARIES_DIR=<dir>/dictionaries` として読みます。たとえば `LINDERA_DATA_DIR=/home/user/.local/share/lindera` なら次のように警告します：

```text
Warning: LINDERA_DATA_DIR is deprecated and will be removed in v8.0.0; set LINDERA_DICTIONARIES_DIR=/home/user/.local/share/lindera/dictionaries instead
```

両方を設定した場合は `LINDERA_DICTIONARIES_DIR` を使い、`LINDERA_DATA_DIR` は無視されると警告します。どちらの変数も、ビルド時のソースキャッシュ用変数 `LINDERA_BUILD_DICTIONARY_CACHE_DIR` や、v7.0.0 で削除されたその旧名 `LINDERA_DICTIONARIES_PATH` とは無関係です。

### RUST_LOG

環境変数 `RUST_LOG` で、CLI が標準エラー出力に書くメッセージ（[警告とログメッセージ](../lindera-cli/commands.md#警告とログメッセージ)を参照）を選べます。書式は [env_logger](https://docs.rs/env_logger/0.11.11/env_logger/#enabling-logging) のものです：

- 指定なし: 警告とエラー
- `RUST_LOG=error`: エラーだけ。警告は出しません。`RUST_LOG=off` はログメッセージを一切出しません
- `RUST_LOG=debug`: デバッグ用のメッセージも出します。たとえば `lindera build` が読み書きするファイル

この設定で標準出力は変わりません。また、失敗したコマンドのエラーは設定にかかわらず出します。標準エラー出力が端末のときはメッセージに色を付けます。`RUST_LOG_STYLE=never` で色を消せます。

## Ruby の gem のビルド

### LINDERA_FEATURES

Ruby の gem のビルド（`gem install lindera` や `bundle exec rake compile`）は、有効にする Cargo の feature のカンマ区切りリストを `LINDERA_FEATURES` から読みます。指定しない場合は、デフォルトの feature（`train`）で、辞書を埋め込まずにビルドします。

```bash
LINDERA_FEATURES="embed-ipadic" gem install lindera
```

feature の一覧は、Ruby バインディングの [Feature フラグ](../lindera-ruby/installation.md#feature-フラグ)を参照してください。

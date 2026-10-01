# v6 から v7 への移行

Lindera v7.0.0 では `lindera` クレートがファサード（facade）になりました。
形態素セグメンター（`lindera-segmenter` として公開）を再エクスポートし、
新しくデフォルト有効になった `analysis` feature を通じて `lindera-analysis` の
分析チェーンを `lindera::analysis` として提供します。依存 1 行で
`Segmenter`・`Tokenizer`・すべてのフィルタが使えます。あわせて
`lindera-binding-core` を `lindera-binding` に改名し、v5.0.0 で予告していた
`LINDERA_DICTIONARIES_PATH` のフォールバックを削除し、IPADIC のスキーマで
`conjugation_type` と `conjugation_form` の名前が逆になっていた誤りを
修正し、MeCab と同様に空白をラティス上で読み飛ばすようにしました。
このガイドでは、すべての破壊的変更とその対処方法を説明します。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| `lindera` がファサードになり、`lindera::analysis` が `lindera-analysis` を再エクスポート（`analysis` feature、デフォルト有効） | `lindera` と `lindera-analysis` の両方に依存する Rust ユーザー | 両方を同時に `"7"` に上げるか、`lindera-analysis` を外して `lindera::analysis::…` から import する |
| `default-features = false` は「セグメンターのみ」の意味になり、`analysis`（と従来どおり `mmap`）が無効になる | `default-features = false` で `lindera::analysis` を使いたい Rust ユーザー | `features` に `"analysis"` を追加する（メモリマップ読み込みを維持するなら `"mmap"` も列挙したままにする） |
| 従来の `lindera` クレートは `lindera-segmenter` として公開 | 直接依存する人はいない | 対応不要 — `lindera = "7"` が再エクスポートする |
| 生の `lindera-dictionary` モジュールを `lindera::dictionary` 配下から参照できる | `lindera-dictionary` を直接使うユーザー | 対応不要 — 任意で直接依存を外し、`lindera::dictionary::core::…`・`::builder`・`::viterbi` などを使う |
| **暗黙の feature `lindera-ipadic`・`lindera-ipadic-neologd`・`lindera-unidic`・`lindera-sudachidict`・`lindera-ko-dic`・`lindera-cc-cedict`・`lindera-jieba` の削除** | `features = [...]` に辞書クレート名を書いて有効化していたユーザー | 代わりに `embed-*` feature を使う |
| **`lindera-binding-core` を `lindera-binding` に改名** | バインディング用ヘルパークレートを使う Rust ユーザー | `lindera-binding` に依存し、`lindera_binding_core::` を `lindera_binding::` に置き換える |
| **`LINDERA_DICTIONARIES_PATH` の削除** | 非推奨のビルドキャッシュ変数をまだ設定しているユーザー | `LINDERA_BUILD_DICTIONARY_CACHE_DIR` を設定する。旧名は無視される |
| **IPADIC・IPADIC-NEologd の `conjugation_type` と `conjugation_form` が正しい列を指すように修正** | IPADIC・IPADIC-NEologd でこの 2 フィールドを名前で読むユーザー（`Token::get`・`Token::as_value`・`lindera tokenize -o json`・バインディングのスキーマ） | 2 つの値が入れ替わることを前提にする。v6.2.0 以前に作った辞書は再ビルドまたは再ダウンロードする |
| **空白をラティス上で読み飛ばし、MeCab と同様に空白の前後の語を直接接続する** | `keep_whitespace` が false（デフォルト）で空白を含むテキストを分割するユーザー。ko-dic で特に顕著 | そのようなテキストは MeCab と同じ分割になることを前提にする。`skip_whitespace(false)`（`"skip_whitespace": false`、`--disable-skip-whitespace`）で v6 の分割に戻せる |

言語バインディング（Python・Node.js・Ruby・PHP・WASM）と CLI の API・パッケージ名は
変わらず、バージョン番号だけが 7.0.0 になります。出力の違いは 2 つあり、どちらも
後述します。IPADIC・IPADIC-NEologd では、`conjugation_type` と
`conjugation_form` という名前で返る値が入れ替わります。また、空白を含む
テキストは MeCab と同じように分割されます。空白を含まないテキストに対しては、
v7.0.0 は v6.2.0 と同じトークンを、同じ位置ベースの詳細情報（details）とともに
出力します。

## `lindera` クレートはファサードに

v6 までの Rust API は、`lindera`（セグメンターと辞書読み込み API）と
`lindera-analysis`（character filter・token filter・`Tokenizer`）の 2 クレートに
分かれており、それぞれに依存する必要がありました。v7 では `lindera` クレートが
両者を束ねる薄いファサードになります:

```text
lindera ──┬─▶ lindera-segmenter ──┬─▶ lindera-dictionary
          │                       ├─▶ 辞書クレート          [features: embed-*]
          │                       └─▶ lindera-trainer     [feature: train]
          └─▶ lindera-analysis    [feature: analysis、デフォルト有効] ──▶ lindera-segmenter
```

公開ツリーは v6 のすべての `lindera::…` パスを維持し、`lindera::analysis` を
追加します:

```text
lindera
├── LinderaResult, get_version()
├── dictionary, error, mode, segmenter, space_penalty, token, worker   （lindera-segmenter から。v6 と同じ）
└── analysis   [feature: analysis、デフォルト有効]   （lindera-analysis: character_filter, token_filter, tokenizer, worker）
```

### 依存は 1 つに

```toml
# v6
[dependencies]
lindera = "6"
lindera-analysis = "6"

# v7
[dependencies]
lindera = "7"
```

```rust
// v6
use lindera_analysis::tokenizer::Tokenizer;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;

// v7
use lindera::analysis::tokenizer::Tokenizer;
use lindera::analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
```

`lindera-analysis = "7"` への直接依存を残しても動作します。`lindera::analysis` は
同じクレートなので、`lindera_analysis::tokenizer::Tokenizer` と
`lindera::analysis::tokenizer::Tokenizer` は同一の型であり、既存の `use` パスを
変える必要はありません。

### `lindera` と `lindera-analysis` は同時に上げる

両方の依存を残す場合は、同じタイミングで両方を上げてください。
`lindera-analysis` 6.x は `lindera` 6.x（旧セグメンター）に依存するため、
`lindera` の行だけを `"7"` にして `lindera-analysis = "6"` を残すと、
セグメンターが 2 バージョン同時にビルドされます:

```toml
# Segmenter 型が 2 つになる: lindera 7（lindera-segmenter 経由）と lindera 6（lindera-analysis 6 経由）
[dependencies]
lindera = "7"
lindera-analysis = "6"
```

この状態では `lindera::segmenter::Segmenter` は v7 の型、
`lindera_analysis::tokenizer::Tokenizer::new` が受け取るのは v6 の型となり、
型不一致でコンパイルに失敗します。両方の行を同時に `"7"` にするか、
前述のとおり `lindera-analysis` を外してください。

### `default-features = false` は「セグメンターのみ」の意味に

ファサードのデフォルト feature は `mmap` と `analysis` です。
`default-features = false` にすると純粋なセグメンターになり、両方が無効に
なります。`lindera::analysis` は存在せず、ファイルシステム辞書のメモリマップ
読み込みもデフォルトではなくなります（v6 でも `default-features = false` は
`mmap` を無効にしていました）。必要なものを個別に有効にしてください:

```toml
# メモリマップ読み込み付きのセグメンター（v6 のデフォルトビルドと同じ）
lindera = { version = "7", default-features = false, features = ["mmap"] }

# セグメンターと分析チェーン。メモリマップ読み込みはデフォルトにしない
lindera = { version = "7", default-features = false, features = ["analysis"] }
```

`memmap2` 自体は `lindera-dictionary` のデフォルト feature を通じてどちらの場合も
リンクされます。ファサードの `mmap` feature は `load_dictionary` などの
`use_mmap` のデフォルト値を決めるだけで、
`load_dictionary_with_options(uri, true)` は feature なしでも動作します。

### セグメンターは `lindera-segmenter` として公開

v6 までの `lindera` クレートは、同じモジュール（`dictionary`・`error`・`mode`・
`segmenter`・`space_penalty`・`token`・`worker`）と同じ API を持つ
`lindera-segmenter` 7.0.0 として続いています。ファサードはそのすべてを
ルートで再エクスポートするため、v6 でコンパイルできた `lindera::…` パスは
v7 でもそのままコンパイルできます。このクレートは、`lindera-analysis` が
ファサードに依存せずにセグメンターを利用できるようにするために存在します。
`lindera-segmenter` に直接依存する必要はなく、アプリケーションコードは
引き続き `lindera` に依存してください。

## `lindera::dictionary` から生の辞書クレートを参照できる

`lindera-dictionary` の構成部品（辞書のデータ構造・辞書ビルダー・Viterbi
ラティスなど）を使うコードは、従来 `lindera-dictionary` への直接依存が
必要でした。v7 ではこれらのモジュールが、高レベルの読み込み API と並んで
`lindera::dictionary` の中に再エクスポートされます:

```text
lindera::dictionary
├── Dictionary, Metadata, Schema, load_dictionary, load_user_dictionary, DictionaryKind, …   （変更なし）
├── core          lindera_dictionary::dictionary（prefix_dictionary, character_definition, connection_cost_matrix, unknown_dictionary, …）
├── builder, error, loader, mode, nbest, space_penalty, util, viterbi
├── embedded_dictionary!, include_bytes_aligned!
└── trainer       lindera_trainer（`train` feature。変更なし）
```

既存の `lindera::dictionary::*` パスはすべて有効なままで、再エクスポートは同じ
アイテムを指します。`lindera::dictionary::Dictionary` と
`lindera::dictionary::core::Dictionary` は同一の型です。`lindera-dictionary` への
直接依存も引き続き動作しますが、必須ではなくなりました:

```rust
// v6: Cargo.toml に lindera-dictionary が必要
use lindera_dictionary::dictionary::prefix_dictionary::PrefixDictionary;
use lindera_dictionary::viterbi::Lattice;

// v7: ファサード経由で参照できる
use lindera::dictionary::core::prefix_dictionary::PrefixDictionary;
use lindera::dictionary::viterbi::Lattice;
```

> [!NOTE]
> `lindera::dictionary::core` は Rust の `core` クレートと同じ名前です。
> `use lindera::dictionary::*;` のような glob import はこのモジュールをスコープに
> 持ち込み、そのモジュール内で `core` クレートを隠してしまうため、
> `core::fmt::Debug` のようなパスが標準ライブラリを指さなくなります。
> 必要なアイテムを名前で個別に import してください。

## 暗黙の辞書 feature の削除

v6 では `lindera` の optional な辞書依存が Cargo の feature としても機能しており、
`features = ["lindera-ipadic"]` という指定が受け付けられていました（辞書を
埋め込まずにクレートだけを取り込む動作でした）。ファサードは辞書クレートに
依存しないため、次の 7 つの暗黙の feature は存在しなくなります:
`lindera-ipadic`・`lindera-ipadic-neologd`・`lindera-unidic`・
`lindera-sudachidict`・`lindera-ko-dic`・`lindera-cc-cedict`・`lindera-jieba`。
これらを指定したマニフェストは Cargo がエラーにします。辞書の埋め込みには、
従来から正式な方法である `embed-*` feature を使ってください:

```toml
# v6
[dependencies]
lindera = { version = "6", features = ["lindera-ipadic"] }

# v7
[dependencies]
lindera = { version = "7", features = ["embed-ipadic"] }
```

## `lindera-binding-core` は `lindera-binding` に改名

共通ヘルパークレートの上に独自の言語バインディングを構築している場合のみ
関係します。`lindera-binding` 7.0.0 は `lindera` と `lindera-analysis` の代わりに
`lindera` ファサード（`analysis` feature 付き）に依存します。クレート自身の
API（`CoreTokenizerBuilder`・`CoreTokenizer`・`TokenView`、および argument・
metadata・schema の各ヘルパー）は変わりません。公開済みの `lindera-binding-core`
（4.0.0〜6.2.0）は crates.io にそのまま残り、yank もされませんが、
以降のバージョンは公開されません。

```toml
# v6
[dependencies]
lindera-binding-core = "6"

# v7
[dependencies]
lindera-binding = "7"
```

```rust
// v6
use lindera_binding_core::tokenizer::CoreTokenizer;

// v7
use lindera_binding::tokenizer::CoreTokenizer;
```

## `LINDERA_DICTIONARIES_PATH` の削除

v5.0.0 でビルド時の辞書キャッシュ変数は `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に
改名され、旧名 `LINDERA_DICTIONARIES_PATH` はビルド警告付きの非推奨
フォールバックとして残されていました（[v4 から v5 への移行](./migration_v4_to_v5.md)
を参照）。v7.0.0 ではこのフォールバックを削除します。旧名だけを設定した
ビルドはキャッシュを使わず、警告も出しません。辞書クレートはキャッシュ
未設定の場合と同じように辞書をダウンロード・ビルドします。設定している
すべての場所（シェルのプロファイル・CI 設定・コンテナイメージ）で変数名を
変更してください:

```shell
# v6
export LINDERA_DICTIONARIES_PATH=/path/to/cache

# v7
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
```

## IPADIC の活用フィールド名の修正

IPADIC は 8 列目に活用型（例: `五段・カ行イ音便`）、9 列目に活用形（例:
`連用タ接続`）を格納しています。v1.0.0 から v6.2.0 までの `lindera-ipadic` と
`lindera-ipadic-neologd` のスキーマは、この 2 列の名前を逆に付けていたため、
名前で引くと常にもう一方の値が返っていました。v7.0.0 で名前を修正しました。
UniDic と SudachiDict は元から正しい順序です。

`書いた` の `書い` の場合:

| フィールド名 | v6 | v7 |
| --- | --- | --- |
| `conjugation_type` | `連用タ接続` | `五段・カ行イ音便` |
| `conjugation_form` | `五段・カ行イ音便` | `連用タ接続` |

変わるのは名前によるアクセスだけです。`Token::get("conjugation_type")` と
`Token::get("conjugation_form")`、`Token::as_value()` と
`lindera tokenize -o json` の JSON、バインディングが公開する辞書スキーマの
フィールド名が該当します。位置によるアクセス（`details`、MeCab 形式と wakati
形式の出力）は変わりません。値そのものは元から正しい列に入っていたためです。
これらのフィールドを名前で読んでいるコードや、回避策として 2 つを入れ替えて
読んでいたコードは修正してください。

フィールド名はライブラリではなく、ビルド済み辞書ごとに保存される
`metadata.json` から読み込まれます。また、辞書フォーマットのバージョンは
変わっていません。埋め込み辞書（`embed-ipadic`・`embed-ipadic-neologd`）と、
7.0.0 の CLI が `lindera download` で取得する辞書は修正後の名前になります。
v6.2.0 以前にビルドまたはダウンロードした辞書ディレクトリは v7.0.0 でも
そのまま読み込めますが、旧い名前のままです。このような辞書ディレクトリは、
次のいずれかで対応してください:

- v7.0.0 の `lindera-ipadic/metadata.json`（または
  `lindera-ipadic-neologd/metadata.json`）で再ビルドするか、7.0.0 の
  リリースアセットをダウンロードする。
- その辞書の `metadata.json` の `dictionary_schema.fields` で
  `"conjugation_form"` と `"conjugation_type"` を入れ替える。辞書データ自体は
  変わらないため、再ビルドは不要です。

IPADIC の `metadata.json` の独自のコピーで辞書をビルドしている場合は、
そのコピーも同じように入れ替えてください。

## 空白をラティス上で読み飛ばす

`keep_whitespace` が false（デフォルト）のとき、v7.0.0 は MeCab と同様に空白を
Viterbi ラティス上で読み飛ばします。空白の後ろの語は空白の前の語に直接
接続します。v6 は空白を出力から除外していましたが、ラティスには `SPACE` の
未知語として残していました。MeCab で学習した辞書ではこの項目との連接は
学習時に一度も現れないため（ko-dic ではその連接コストがすべて 0）、空白の
たびに前後の語の文脈が途切れていました。

出力が変わるのは空白を含む文だけです。分割の区切りになった `\n` や `\t` で
終わる文（`。` で終わらない行など）もこれに含まれ、その文の最後の語は文末に
直接接続するようになります。

| 辞書 | 影響 | 例 |
| --- | --- | --- |
| ko-dic | 空白を含む韓国語が mecab-ko と同じ解析になる | `2년 전 대회` の `전` は `저/NP + ㄴ/JX` ではなく `NNG`、`하고 있다` の `있` は `VV` ではなく `VX` |
| IPADIC・IPADIC-NEologd・UniDic | MeCab と同様に、半角空白の後ろの語が空白の前の語に直接接続する | IPADIC の `Google が 新しい` の `が` は接続詞ではなく格助詞、`東京 都` の `都` は MeCab と同様に接尾 |
| SudachiDict | 変わらない。Sudachi は空白をラティスに残し、コストもそれを前提にしているため、`metadata.json` で `skip_whitespace` を `false` にしている | — |
| CC-CEDICT・Jieba | 最良パスは変わらない（連接コストを持たない辞書のため）。N-best のコストに空白ノードの分が含まれなくなる | — |

デフォルトは辞書の `metadata.json` で決まり、`skip_whitespace` を設定していない
辞書は読み飛ばします。Lindera 6.x でビルドした SudachiDict にはこの設定がない
ため、再ビルドするか `metadata.json` に設定を追加するまでは空白を読み飛ばします。

トークンの表層形とオフセットには読み飛ばした空白が含まれず、空白を含まない
テキストの分割は従来とまったく同じです。`keep_whitespace(true)` では空白が
ラティスに残り、出力は変わりません。

v6 の分割結果に戻すには、読み飛ばしを無効にします。空白は引き続き出力から
除外されます。設定で `skip_whitespace` を省略するか `null` にすると、辞書の
デフォルトのままになります:

```rust
// Segmenter
let segmenter = Segmenter::new(Mode::Normal, dictionary, None).skip_whitespace(false);
// SegmentWorker
worker.set_skip_whitespace(false);
```

```yaml
# 設定ファイル / TokenizerBuilder::set_segmenter_skip_whitespace(false)
segmenter:
  skip_whitespace: false
```

```sh
lindera tokenize --disable-skip-whitespace
```

言語バインディングでは設定ファイルで指定します。

## 対応が不要なケース

- **言語バインディングと CLI のユーザー**: Python・Node.js・Ruby・PHP・WASM の
  各パッケージと `lindera-cli` の API・パッケージ名は変わりません。今回の
  構成変更は Rust クレート内部のものです。出力の変化は、この 2 フィールドを
  名前で読む場合にのみ影響する IPADIC の活用フィールドの修正と、空白を含む
  テキストの分割です。
- **`lindera = "6"` のまま使い続けるプロジェクト**: `lindera` と `lindera-analysis`
  の 6.x は crates.io に残り、組み合わせて動作し続けます。メジャーバージョンを
  上げるまで何も変わりません。
- **セグメンター API だけを使うユーザー**: `Segmenter`・`load_dictionary`・`Mode`
  など v6 の `lindera::…` アイテムを使うコードは、`lindera = "7"` でそのまま
  コンパイルできます。ただしデフォルトビルドでは `lindera-analysis` とその依存
  （kanaria・regex・serde_yaml_ng・unicode-blocks・unicode-normalization・
  unicode-segmentation）もコンパイルされるため、v6 の依存ツリーを維持したい
  場合は `default-features = false, features = ["mmap"]` を指定してください。

## アップグレードチェックリスト

Rust クレートのユーザー:

- `lindera` を `"7"` に上げ、同じコミットで `lindera-analysis` も `"7"` に上げるか、
  取り除いて `lindera::analysis::…` から import する。
- 任意で `use` パスの `lindera_analysis::` を `lindera::analysis::` に置き換える
  （どちらも同じ型を指す）。
- `default-features = false` を指定している場合は、必要に応じて `features` に
  `"analysis"`（`Tokenizer` とフィルタ）と `"mmap"`（メモリマップ読み込み）を
  追加する。
- `features = ["lindera-<辞書>"]` を `features = ["embed-<辞書>"]` に置き換える。
- 任意で `lindera-dictionary` への直接依存を外し、`lindera::dictionary::core`・
  `::builder`・`::viterbi` などの再エクスポートに切り替える。

バインディングの作者:

- `lindera-binding-core` の代わりに `lindera-binding` に依存し、
  `lindera_binding_core::` を `lindera_binding::` に置き換える。

ビルド環境:

- シェルのプロファイル・CI 設定・コンテナイメージで `LINDERA_DICTIONARIES_PATH`
  を `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に改名する。

IPADIC・IPADIC-NEologd のユーザー:

- `conjugation_type` や `conjugation_form` を名前で読んでいる場合（CLI の JSON
  出力を含む）は、2 つの値が入れ替わることを前提にする。
- v6.2.0 以前に作った辞書ディレクトリは、再ビルド・再ダウンロードするか、
  `metadata.json` の 2 つの名前を入れ替える。

空白を含むテキストを分割するすべてのユーザー:

- そのようなテキストは MeCab と同じ分割になることを前提にする（ko-dic で特に
  顕著）。v6 の出力が必要な場合は `skip_whitespace(false)`、
  `"skip_whitespace": false`、`--disable-skip-whitespace` を指定する。

言語バインディングと CLI:

- 7.0.0 リリースを取り込む以外に対応は不要（上記の IPADIC と空白の項目を除く）。

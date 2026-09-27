# v6 から v7 への移行

Lindera v7.0.0 では `lindera` クレートがファサード（facade）になりました。
形態素セグメンター（`lindera-segmenter` として公開）を再エクスポートし、
新しくデフォルト有効になった `analysis` feature を通じて `lindera-analysis` の
分析チェーンを `lindera::analysis` として提供します。依存 1 行で
`Segmenter`・`Tokenizer`・すべてのフィルタが使えます。あわせて
`lindera-binding-core` を `lindera-binding` に改名し、v5.0.0 で予告していた
`LINDERA_DICTIONARIES_PATH` のフォールバックを削除しました。このガイドでは、
すべての破壊的変更とその対処方法を説明します。

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

言語バインディング（Python・Node.js・Ruby・PHP・WASM）と CLI は影響を受けません。
API・パッケージ名・出力は変わらず、バージョン番号だけが 7.0.0 になります。
トークナイズ結果も不変です — v7.0.0 は構成変更のみのリリースであり、同じ入力と
辞書に対して v6.2.0 とバイト単位で同一のトークンを出力します。

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

## 対応が不要なケース

- **言語バインディングと CLI のユーザー**: Python・Node.js・Ruby・PHP・WASM の
  各パッケージと `lindera-cli` は影響を受けません。今回の構成変更は Rust
  クレート内部のもので、API・パッケージ名・出力は変わりません。
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

言語バインディングと CLI:

- 7.0.0 リリースを取り込む以外に対応は不要。

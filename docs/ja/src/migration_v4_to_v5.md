# v4 から v5 への移行

Lindera v5.0.0 では、ワークスペースをリーンなコアを中心に再構成しました。
`lindera` クレートのデフォルトビルドは純粋な形態素分割器（セグメンター）となり、
辞書学習パイプラインは独立したクレートに移動しました。このガイドでは、
すべての破壊的変更とその対処方法を説明します。

> [!NOTE]
> v5.0.0 は次期リリース予定であり、まだ crates.io には公開されていません。現時点での公開バージョンは
> `4.0.1` です。以下で説明する変更は、バージョン更新に先立って `main` ブランチには既に存在します。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| 分析チェーンが新クレート `lindera-analysis` へ移動 | `Tokenizer`・character filter・token filter を使う Rust ユーザー | `lindera-analysis` に依存し import パスを更新 |
| `lindera-dictionary` の `train` feature 廃止 | `lindera-dictionary --features train` を直接使うユーザー | `lindera-trainer`（または `lindera` facade の `train` feature）に依存 |
| ビルドキャッシュ変数の改名 | `LINDERA_DICTIONARIES_PATH` を設定しているユーザー | `LINDERA_BUILD_DICTIONARY_CACHE_DIR` に改名（旧名は v6.0.0 まで動作） |

言語バインディング（Python・Node.js・Ruby・PHP・WASM）と CLI は影響を
受けません。必要な feature はそれぞれが有効化しており、API と出力も
変わりません。トークナイズ結果も不変です — 同じ入力と辞書に対して、
v5 は v4 とバイト単位で同一のトークンを出力します。

## 分析チェーンは lindera-analysis クレートへ

v5.0 では `lindera` クレートは純粋な形態素分割器になりました。
`character_filter`・`token_filter`・`tokenizer` モジュールは新クレート
[`lindera-analysis`](https://crates.io/crates/lindera-analysis) に移動しています
（Lucene のトークナイザーコアとアナライザーモジュールの分離と同じ構成です）。

`Tokenizer` やフィルタを使用している場合は、新クレートに依存し import パスを
更新してください。API 自体は変更ありません：

```toml
# v4
[dependencies]
lindera = "4.0"

# v5
[dependencies]
lindera = "5.0"
lindera-analysis = "5.0"
```

```rust
// v4
use lindera::tokenizer::Tokenizer;
use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;

// v5
use lindera_analysis::tokenizer::Tokenizer;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
```

テキストの分割だけを行う場合、コードの変更は不要です。むしろ依存ツリーが
軽量化されます（kanaria・unicode-normalization・unicode-segmentation・
unicode-blocks・serde_yaml_ng がビルドされなくなります）：

```rust
use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

let dictionary = load_dictionary("/path/to/ipadic")?;
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
let tokens = segmenter.segment(Cow::Borrowed("関西国際空港限定トートバッグ"))?;
```

## 学習機能は lindera-trainer クレートへ

CRF 学習パイプライン（`TrainerConfig`・`Trainer`・`Corpus`・`Model`・
`SerializableModel`）は、`lindera-dictionary` の `train` feature 配下の
`trainer` モジュールから、新しい `lindera-trainer` クレートに移動しました。
これにより `lindera-dictionary` は `lindera-crf` と `regex` に依存しなく
なりました。

**`lindera` facade 経由の利用は変更不要です** — `train` feature が
`lindera-trainer` を取り込み、同じパスで再エクスポートします：

```rust
// v4 でも v5 でも動作します（train feature 有効時）：
use lindera::dictionary::trainer::{Corpus, Trainer, TrainerConfig};
```

`lindera-dictionary --features train` を直接使っていた場合のみ、
切り替えが必要です：

```toml
# v4
[dependencies]
lindera-dictionary = { version = "4.0", features = ["train"] }

# v5
[dependencies]
lindera-dictionary = "5.0"
lindera-trainer = "5.0"
```

```rust
// v4
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// v5
use lindera_trainer::{Corpus, Trainer, TrainerConfig};
```

CLI の `lindera train` → `lindera export` → `lindera build` ワークフローに
変更はありません。

## ビルドキャッシュ環境変数の改名

ビルド時の辞書キャッシュ変数 `LINDERA_DICTIONARIES_PATH` は
`LINDERA_BUILD_DICTIONARY_CACHE_DIR` に改名されました。この変数は辞書クレートの
build script だけがビルド時に読み取るもので、ダウンロードした辞書アーカイブと
ビルド済みバイナリ辞書を保持する自動管理のキャッシュを指定します。新しい名前は
その契約（ビルド時専用・辞書・キャッシュ）を明示します。

旧名は v5.x の間は非推奨のフォールバックとして動作し（両方設定時は新名が優先）、
v6.0.0 で削除されます。

```shell
# v4
export LINDERA_DICTIONARIES_PATH=/path/to/cache

# v5
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
```

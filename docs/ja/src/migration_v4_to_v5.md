# v4 から v5 への移行

Lindera v5.0.0 では、ワークスペースをリーンなコアを中心に再構成しました。
`lindera` クレートのデフォルトビルドは純粋な形態素分割器（セグメンター）となり、
辞書学習パイプラインは独立したクレートに移動しました。このガイドでは、
すべての破壊的変更とその対処方法を説明します。

## 概要

| 変更 | 影響範囲 | 対処方法 |
| --- | --- | --- |
| `analysis` がデフォルト feature から除外 | `Tokenizer`・character filter・token filter を使う Rust ユーザー | lindera の feature リストに `analysis` を追加 |
| `lindera-dictionary` の `train` feature 廃止 | `lindera-dictionary --features train` を直接使うユーザー | `lindera-trainer`（または `lindera` facade の `train` feature）に依存 |

言語バインディング（Python・Node.js・Ruby・PHP・WASM）と CLI は影響を
受けません。必要な feature はそれぞれが有効化しており、API と出力も
変わりません。トークナイズ結果も不変です — 同じ入力と辞書に対して、
v5 は v4 とバイト単位で同一のトークンを出力します。

## デフォルトが純粋なセグメンターに

`analysis` cargo feature（v4.1.0 で導入）は分析チェーン
（`character_filter`・`token_filter`・`tokenizer` モジュール）をゲートします。
v5.0 ではデフォルト feature セットから除外されたため、デフォルトビルドは
`Segmenter` API のみを提供します。

`Tokenizer` やフィルタを使用している場合は、feature を追加してください：

```toml
# v4
[dependencies]
lindera = "4.0"

# v5
[dependencies]
lindera = { version = "5.0", features = ["analysis"] }
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

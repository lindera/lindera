# モジュール構成の変更提案: `lindera` ファサードの導入

作成日: 2026-09-24
対象バージョン: 6.x（後述の通り、公開パスを維持すればマイナーリリースで出せる）

## 1. 目的

利用者の `Cargo.toml` に次の一行を書くだけで、`lindera-*` の全クレートを利用できるようにする。

```toml
[dependencies]
lindera = "6"
```

そのうえで、これまで通りファサード（`lindera`）の下のモジュールパスで各クレートの API にアクセスできるようにする。

```rust
use lindera::dictionary::Metadata;
use lindera::analysis::tokenizer::Tokenizer;
```

v6.0 で `lindera` と `lindera-analysis` に分割したことで「セグメンタだけなら `lindera`、フィルタも使うなら `lindera-analysis` も追加」という二段構えになっていた。この提案は、内部の分離を保ったまま「一つ入れれば全部使える」体験を取り戻すものである。

## 2. 変更の概要

| 項目 | 現在 | 変更後 |
| --- | --- | --- |
| セグメンタ本体 | `lindera` | `lindera-segmenter`（リネーム） |
| ファサード | なし | `lindera`（新規。再エクスポートと feature 転送のみ） |
| バインディング共通ヘルパー | `lindera-binding-core` | `lindera-binding`（リネーム） |
| `lindera-analysis` の依存先 | `lindera` | `lindera-segmenter` |
| CLI・各バインディングの依存先 | `lindera` + `lindera-analysis`（wasm は `lindera-dictionary` も） | `lindera` のみ（バインディングは `lindera-binding` も） |

crates.io 上で `lindera-segmenter` と `lindera-binding` はどちらも未使用であることを確認済み（2026-09-24 時点）。

## 3. 現在の依存関係（v6.0.0）

`[opt]` は optional 依存で feature により有効化されるもの、`[build]` は build-dependencies。

```
lindera-cli ─────────┬─▶ lindera ───────────┬─▶ lindera-dictionary
                     │                      ├─▶ lindera-ipadic          [opt: embed-ipadic]
lindera-python ──┐   │                      ├─▶ lindera-ipadic-neologd  [opt: embed-ipadic-neologd]
lindera-nodejs ──┤   │                      ├─▶ lindera-unidic          [opt: embed-unidic]
lindera-ruby ────┼───┤                      ├─▶ lindera-sudachidict     [opt: embed-sudachidict]
lindera-php ─────┤   │                      ├─▶ lindera-ko-dic          [opt: embed-ko-dic]
lindera-wasm ────┘   │                      ├─▶ lindera-cc-cedict       [opt: embed-cc-cedict]
   │                 │                      ├─▶ lindera-jieba           [opt: embed-jieba]
   │                 │                      └─▶ lindera-trainer         [opt: train]
   │                 │                              ├─▶ lindera-crf
   │                 │                              └─▶ lindera-dictionary
   │                 └─▶ lindera-analysis ─▶ lindera
   │
   ├─▶ lindera-binding-core ─┬─▶ lindera
   │                         └─▶ lindera-analysis
   └─▶ lindera-dictionary   (wasm だけ直接参照している)

lindera-{ipadic,ipadic-neologd,unidic,sudachidict,ko-dic,cc-cedict,jieba}
   ─▶ lindera-dictionary  (通常依存 + [build] build_rs feature)
```

## 4. 変更後の依存関係

```
利用者の Cargo.toml: lindera = "6"
        │
        ▼
lindera (facade)  ← 再エクスポートと feature 転送だけの薄いクレート
   ├─▶ lindera-segmenter ─────────┬─▶ lindera-dictionary
   │      (旧 lindera)            ├─▶ lindera-ipadic          [opt: embed-ipadic]
   │                              ├─▶ lindera-ipadic-neologd  [opt: embed-ipadic-neologd]
   │                              ├─▶ lindera-unidic          [opt: embed-unidic]
   │                              ├─▶ lindera-sudachidict     [opt: embed-sudachidict]
   │                              ├─▶ lindera-ko-dic          [opt: embed-ko-dic]
   │                              ├─▶ lindera-cc-cedict       [opt: embed-cc-cedict]
   │                              ├─▶ lindera-jieba           [opt: embed-jieba]
   │                              └─▶ lindera-trainer         [opt: train]
   │                                      ├─▶ lindera-crf
   │                                      └─▶ lindera-dictionary
   ├─▶ lindera-analysis  [opt: analysis, default on]
   │      └─▶ lindera-segmenter
   ├─▶ lindera-dictionary   (生の低レベル API を別名で出す場合)
   └─▶ lindera-trainer      [opt: train]  (lindera::trainer として出す場合)

lindera-cli ────────────▶ lindera (facade)

lindera-python ─┐
lindera-nodejs ─┤
lindera-ruby ───┼──┬───▶ lindera (facade)
lindera-php ────┤  └───▶ lindera-binding  (旧 lindera-binding-core)
lindera-wasm ───┘              └───▶ lindera (facade)

lindera-{ipadic,ipadic-neologd,unidic,sudachidict,ko-dic,cc-cedict,jieba}
   ─▶ lindera-dictionary  (変更なし)
```

### 要点

- ファサードの直下に入るのは `lindera-segmenter` と `lindera-analysis` の 2 本だけ。辞書クレートと trainer は今と同じく `lindera-segmenter` の optional 依存で、ファサードは `embed-*` と `train` を転送するだけ。
- `lindera-analysis` の参照先を `lindera` から `lindera-segmenter` に変えるのが唯一の必須変更。これをしないと `lindera` ⇄ `lindera-analysis` で循環する。
- 上位 7 クレート（`lindera-cli`、`lindera-binding`、5 つのバインディング）は `lindera` 一本に集約できる。
- `lindera-dictionary` の位置と、辞書クレートの `[build]` 依存は変わらない。

## 5. ファサードが公開するモジュールツリー

```
lindera
├── LinderaResult                （lindera_segmenter::LinderaResult）
├── get_version()
├── dictionary                   （lindera_segmenter::dictionary をそのまま再エクスポート）
│   ├── Dictionary, Metadata, UserDictionary, Schema, FieldDefinition, FieldType
│   ├── DictionaryBuilder, DictionaryConfig, UserDictionaryConfig
│   ├── DictionaryKind, DictionaryScheme, load_dictionary, ...
│   └── trainer                  [feature: train]（lindera_trainer）
├── error                        （LinderaError, LinderaErrorKind）
├── mode                         （Mode, Penalty）
├── segmenter                    （Segmenter）
├── space_penalty                （SpacePenaltyConfig）
├── token                        （Token）
├── worker
├── analysis                     [feature: analysis, default on]（lindera_analysis）
│   ├── character_filter
│   ├── token_filter
│   ├── tokenizer                （Tokenizer, TokenizerBuilder）
│   └── worker
└── dictionary_core              （lindera_dictionary。名前は要検討）
    ├── dictionary
    ├── loader
    ├── builder
    └── viterbi
```

`lindera::dictionary` は `lindera-dictionary` クレートではなく、現在の `lindera/src/dictionary.rs`（`DictionaryScheme`、`load_dictionary` などの高レベル API と型エイリアス群）を指す。ここを取り違えると `lindera::dictionary::Metadata` が壊れる（`lindera::dictionary::dictionary::metadata::Metadata` になる）。

### `lindera/src/lib.rs` の骨子

```rust
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(inline)]
pub use lindera_segmenter::{
    LinderaResult, dictionary, error, mode, segmenter, space_penalty, token, worker,
};

#[cfg(feature = "analysis")]
#[cfg_attr(docsrs, doc(cfg(feature = "analysis")))]
#[doc(inline)]
pub use lindera_analysis as analysis;

/// `lindera-dictionary` の低レベル API。通常は `lindera::dictionary` で足りる。
#[doc(inline)]
pub use lindera_dictionary as dictionary_core;

#[cfg(feature = "train")]
#[cfg_attr(docsrs, doc(cfg(feature = "train")))]
#[doc(inline)]
pub use lindera_trainer as trainer;

pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

### `lindera/Cargo.toml` の feature 転送

```toml
[features]
default = ["mmap", "analysis"]
analysis = ["dep:lindera-analysis"]
mmap = ["lindera-segmenter/mmap"]
train = ["lindera-segmenter/train", "dep:lindera-trainer"]
ctxfreq = ["lindera-segmenter/ctxfreq"]
embed-ipadic = ["lindera-segmenter/embed-ipadic", "lindera-analysis?/embed-ipadic"]
embed-ipadic-neologd = ["lindera-segmenter/embed-ipadic-neologd", "lindera-analysis?/embed-ipadic-neologd"]
embed-unidic = ["lindera-segmenter/embed-unidic", "lindera-analysis?/embed-unidic"]
embed-sudachidict = ["lindera-segmenter/embed-sudachidict", "lindera-analysis?/embed-sudachidict"]
embed-ko-dic = ["lindera-segmenter/embed-ko-dic", "lindera-analysis?/embed-ko-dic"]
embed-cc-cedict = ["lindera-segmenter/embed-cc-cedict", "lindera-analysis?/embed-cc-cedict"]
embed-jieba = ["lindera-segmenter/embed-jieba", "lindera-analysis?/embed-jieba"]
embed-cjk  = ["embed-ipadic", "embed-ko-dic", "embed-jieba"]
embed-cjk2 = ["embed-unidic", "embed-ko-dic", "embed-jieba"]
embed-cjk3 = ["embed-ipadic-neologd", "embed-ko-dic", "embed-jieba"]
embed-cjk4 = ["embed-sudachidict", "embed-ko-dic", "embed-jieba"]

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

## 6. 設計上の判断と注意点

1. **`lindera::dictionary` の互換性を最優先する。**
   `pub use lindera_segmenter::dictionary;` とし、生の `lindera-dictionary` は `dictionary_core` のような別名で出す。名前は要検討。

2. **循環依存の回避。**
   `lindera-analysis` と `lindera-trainer` のような下位クレートは `lindera-segmenter` を直接参照し、`lindera-cli` と各バインディングはファサード経由に統一する。

3. **`analysis` は optional feature（default on）にする。**
   `lindera-analysis` は `kanaria`、`unicode-normalization`、`unicode-segmentation`、`serde_yaml_ng`、`regex` を引き込む。セグメンタだけ欲しい利用者は `default-features = false` で切れるようにし、v6 で分割した意味を保つ。

4. **semver。**
   公開パスを全て保持し、追加分（`lindera::analysis` など）だけならマイナーリリース（6.1）で出せる。`lindera-segmenter` と `lindera-binding` は新規クレート名なので crates.io 上に公開が必要。

5. **docs.rs の見え方。**
   再エクスポートに `#[doc(inline)]` を付けないと、docs.rs ではリンクだけになり利用者がたらい回しになる。`all-features = true` も合わせて設定する。あわせて `lindera-segmenter/src/dictionary.rs` の `pub type Metadata = ...` 群は `pub use` に変えると rustdoc で元の型に直接飛べる。

6. **ファサードの範囲を広げすぎない。**
   `lindera-cli`（バイナリ）と `lindera-binding`（バインディング用ヘルパー）はファサードに含めない。`lindera-crf` を `train` 配下で出すかは任意だが、利用者が直接触る場面は少ない。

7. **旧 `lindera-binding-core` の扱い。**
   crates.io はクレートの削除も改名もできないので 6.0.0 は残る。yank はせず、README に改名の旨を書いて放置する。yank すると 6.0.0 のバインディングをソースからビルドする人が困る。

## 7. 影響範囲

### 公開順序

```
lindera-crf
→ lindera-dictionary
→ lindera-trainer
→ lindera-cc-cedict, lindera-jieba, lindera-ipadic, lindera-ipadic-neologd,
  lindera-ko-dic, lindera-unidic, lindera-sudachidict
→ lindera-segmenter
→ lindera-analysis
→ lindera
→ lindera-binding
→ lindera-cli
```

`Makefile` の `CARGO_CRATES` と `PUBLISH_CRATE` の行、`.github/workflows/release.yml` の publish ループをこの順序に合わせる。

### 変更が必要なファイル

| 分類 | 対象 |
| --- | --- |
| ワークスペース | `Cargo.toml`（`members`、`workspace.dependencies`）、`Cargo.lock` |
| リネーム | `lindera/` → `lindera-segmenter/`、`lindera-binding-core/` → `lindera-binding/`、新規 `lindera/` |
| 依存先の変更 | `lindera-analysis/Cargo.toml`、`lindera-cli/Cargo.toml`、`lindera-{python,nodejs,ruby,php,wasm}/Cargo.toml` |
| ソース | `lindera-analysis/src/**`（`use lindera::` → `use lindera_segmenter::`）、5 つのバインディングの `use lindera_binding_core::` → `use lindera_binding::` |
| ビルド・公開 | `Makefile`（`CARGO_CRATES`、`FEATURES_*`、`PUBLISH_CRATE`）、`.github/workflows/release.yml`、`periodic.yml`、`regression.yml`、`scripts/bump-up-version.sh` |
| ドキュメント | `README.md`、`README_ja.md`、`docs/src/` と `docs/ja/src/` の `architecture.md`、`development/project_structure.md`、`development/contributing.md`、`lindera-analysis/architecture.md` |
| docコメント | `lindera-analysis/src/worker.rs`、`lindera-dictionary/src/dictionary/connection_cost_matrix.rs`（`lindera-binding-core` を文中で参照） |
| その他 | `lindera-nodejs/index.d.ts`、`scripts/benchmarks/README.md`、`lindera-binding-core/benches/bench_binding_core_ipadic.rs`（ファイル名） |

`lindera-binding-core` への参照はクレート外に約 120 箇所ある。`lindera-segmenter` へのリネームも同じファイル群に触るので、両方を一つの PR にまとめると差分の見通しが良い。

## 8. 進め方

1. `lindera/` を `lindera-segmenter/` に、`lindera-binding-core/` を `lindera-binding/` にリネームし、ワークスペース内の参照先を全て更新する。
2. 新しい `lindera/` にファサードを作る。`lib.rs` は再エクスポートと `get_version()` のみ、`Cargo.toml` は feature 転送のみ。
3. `lindera-cli` と各バインディングを `use lindera::...` のままコンパイルを通す。これがそのまま公開パスの回帰テストになる。
4. `Makefile`、CI、ドキュメントの参照とクレート順序を更新する。
5. `cargo doc --all-features` で docs.rs 上の見え方（inline 再エクスポート、feature バッジ）を確認する。

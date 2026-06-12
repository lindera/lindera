# Lindera コードベース全面リファクタリング計画

本ドキュメントは、ワークスペース全体(18 クレート、Rust ソース約 177 ファイル)の調査に基づく、
フェーズ分けされたリファクタリング計画である。各フェーズは独立してマージ可能な単位に分割し、
「常にグリーンな CI を維持したまま段階的に進める」ことを大原則とする。

---

## 調査で判明した技術的負債の全体像

### A. 辞書クレート 6 個がほぼ完全なコピペクローン

`lindera-ipadic` / `lindera-ipadic-neologd` / `lindera-unidic` / `lindera-ko-dic` /
`lindera-cc-cedict` / `lindera-jieba` の 6 クレートは、`build.rs`(41 行)・
`src/lib.rs`(9 行)・`src/embedded.rs`(88〜96 行)が **97% 以上同一**。

- 実際に異なるのは: 辞書 URL、MD5 ハッシュ、アーカイブ名、ダミー入力、feature フラグ名のみ
  (ロジック差分は jieba の `src_subdir: Some("dict-src")` の 1 箇所だけ)
- 合計約 590 行の重複。マクロ/struct 名(`EmbeddedIPADICLoader` 等)の違いは不必要なバリエーション
- `VERERSION` というタイポ(正しくは `VERSION`)が全 6 クレート + `lindera-dictionary/src/lib.rs:17` +
  `lindera/src/lib.rs:16` にコピペで伝播している
- 全 6 クレートで `anyhow` / `byteorder` / `csv` が通常依存として宣言されているが未使用
  (build-dependencies としてのみ必要)

### B. 言語バインディング 5 種に共有レイヤーがゼロ

`lindera-python` / `lindera-php` / `lindera-ruby` / `lindera-nodejs` / `lindera-wasm`
(合計約 11,750 行)は、それぞれが独立に同じラッパーを再実装している。

| モジュール | Python | PHP | Ruby | Node.js | WASM | ロジック類似度 |
|---|---|---|---|---|---|---|
| tokenizer.rs | 361 | 310 | 313 | 239 | 548 | 80–90% |
| schema.rs | 579 | 509 | 641 | 430 | 452 | 85%+ |
| metadata.rs | 447 | 389 | 433 | 425 | 197 | 80%+ |
| util/convert(値変換) | 115 | 87 | 159 | 53 | 0 | 70–75% |

- 重複は推定 **2,000 行超**。コア API の変更が 5 箇所への追従を要求する
- API の不一致も発生済み:
  - `Token.details` が PHP では `Vec<String>`、他は `Option<Vec<String>>`
  - ビルダーの戻り値が Python はフルーエント(`PyRefMut<Self>`)、他は in-place 変更
  - エラー処理パターンが 5 通り(クラス / 関数ヘルパー / JsValue ラップ)
  - `character_filter` / `token_filter` / `segmenter` モジュールの有無がバインディングごとにバラバラ
    (PHP・Node.js には segmenter なし)

### C. コアクレートの肥大化と重複

- **500 行超のファイルが 7 つ**:
  - `lindera/src/segmenter.rs` — 1,882 行
  - `lindera-dictionary/src/trainer/model.rs` — 1,248 行
  - `lindera-dictionary/src/viterbi.rs` — 1,147 行(`Lattice::set_text()` が約 220 行の巨大関数)
  - `lindera-dictionary/src/trainer.rs` — 971 行
  - `lindera-dictionary/src/trainer/config.rs` — 793 行
  - `lindera-dictionary/src/builder/prefix_dictionary.rs` — 706 行
  - `lindera/src/token_filter/japanese_number.rs` — 1,257 行
- **タグフィルタ 4 種の重複**: `japanese_keep_tags.rs`(479 行)/ `japanese_stop_tags.rs`(437 行)/
  `korean_keep_tags.rs`(384 行)/ `korean_stop_tags.rs`(446 行)は keep/stop の真偽が反転しているだけの
  同一ロジック。約 800 行 → 200 行程度に集約可能
- **ローダーの定型コード重複**: `lindera-dictionary/src/loader/` 配下 5 ファイルが
  `load()` / `load_mmap()` の同一パターンを繰り返す(約 200 行 → 80 行)。
  `loader.rs` の `DictionaryLoader` トレイトは定義されているのにほぼ未使用
- **`lindera/src/dictionary.rs`**: 6 辞書 × `#[cfg(feature)]` の条件付き import が 12 個、
  同一の `#[cfg(any(...))]` 6 連 feature 条件が 3 回以上繰り返し。
  217–245 行にはコメントアウトされたエラー処理(全 6 辞書分)が放置

### D. エラー処理の不統一と `unwrap()` 散在

- `unwrap`/`expect` 合計: lindera 384 / lindera-dictionary 136 / lindera-crf 37+(多くはテストだが、非テストコードにも相当数)
- 非テストコードの問題箇所:
  - `lindera-dictionary/src/builder.rs:60,73,89,101,148` — ビルダー呼び出しに `.unwrap()` 連鎖
  - `lindera-dictionary/src/builder/prefix_dictionary.rs` — CSV フィールドパースに `unwrap()` 13 箇所
  - `lindera-dictionary/src/trainer/feature_extractor.rs` — `Regex::new().unwrap()` 3 箇所 + capture の `unwrap()` 9 箇所以上
  - `lindera-dictionary/src/assets.rs:257,262` — 環境変数の `unwrap()`
- `LinderaError`(`LinderaErrorKind` 41 バリアント)と `anyhow` の併用で戦略が不統一
- CLI では `.map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?` パターンが 15 回以上反復

### E. 公開 API のカプセル化不足

- `lindera-dictionary/src/viterbi.rs` の内部構造体が全フィールド `pub`:
  `WordId { id, is_system, lex_type }`、`WordEntry`、`Edge { path_cost, ... }`、`PathEntry`、
  `Lattice` の内部バッファまで露出。利用側が実装詳細に依存できてしまう
- `WordEntry` のシリアライズ詳細(`SERIALIZED_LEN` 等)も公開

### F. ビルド基盤・CI・リポジトリ衛生

- **Makefile(458 行)**: 13 クレート × clean/format/lint/test/build のほぼ同一ターゲットを手書き反復
- **GitHub Actions(計 2,028 行)**:
  - `release.yml`(1,103 行)と `regression.yml`(495 行)にほぼ同一のテストジョブ 13 個ずつが二重定義
  - Ruby/PHP/Node.js のリリースジョブはプラットフォームマトリクス含め 90% 以上のクローン
  - `cargo metadata | jq` によるバージョン検出が 5 箇所以上で反復
- **生成物のコミット**: `docs/book/` と `docs/ja/book/` の mdBook 生成 HTML/JS 約 21MB
  (`mermaid.min.js` 2.9MB × 2、`searchindex.js` 1.6MB 等)が git 管理下。`.gitignore` 未整備
- `resources/bocchan.txt`(308KB)がリポジトリ内に 2 重コピー
- 後方互換シム: `LINDERA_CACHE` 環境変数の非推奨サポート(`assets.rs:238`)、
  ユーザー辞書の「5-bit variant-count encoding」旧形式互換(`viterbi.rs:409,953`)

### G. テスト体制

- 合計 278 個のインラインテスト。`tests/` ディレクトリによる統合テストはゼロ
- 辞書クレート 6 個と CLI はテストゼロ
- リファクタリングの安全網としては「トークナイズ結果のスナップショット(ゴールデン)テスト」が不在

---

## リファクタリング方針

1. **挙動を変えるリファクタリングと変えないリファクタリングを混ぜない**。各 PR はどちらか一方
2. **フェーズ 0 で安全網を先に作る**。ゴールデンテストとベンチマーク基準値なしに viterbi / segmenter には触らない
3. **semver を尊重する**。フェーズ 1〜5 は非破壊(patch/minor)、破壊的変更はフェーズ 6 (v4.0.0) に集約
4. 各フェーズの PR は **小さく、レビュー可能なサイズ**(目安: 差分 ±1,000 行以内)に分割

---

## フェーズ 0: 安全網の構築(挙動変更なし)

**目的**: 以降の全フェーズで「壊していないこと」を機械的に検証できる状態を作る。

| # | 作業 | 詳細 |
|---|---|---|
| 0-1 | ゴールデン(スナップショット)テスト追加 | `resources/bocchan.txt` 等を入力に、全 6 辞書 × Normal/Decompose モードのトークナイズ結果をスナップショット化(`insta` クレート推奨)。`lindera/tests/` に統合テストとして配置 |
| 0-2 | CLI のスモークテスト | `tokenize` / `build` / `list` サブコマンドの E2E テスト(現状 CLI はテストゼロ) |
| 0-3 | ベンチマーク基準値の記録 | 既存 `bench` ターゲットを実行し、tokenize スループットの基準値を記録。CI での回帰検出は任意(criterion の比較で十分) |
| 0-4 | カバレッジ計測の導入(任意) | `cargo llvm-cov` で現状値を記録し、フェーズごとの劣化を監視 |

- **リスク**: ほぼなし(追加のみ)
- **完了条件**: 全辞書のゴールデンテストが CI で実行され、グリーン
- **規模感**: 小(1〜2 PR)

---

## フェーズ 1: 低リスクの即時クリーンアップ(挙動変更なし)

**目的**: 議論の余地がない無駄を先に一掃し、以降の差分ノイズを減らす。

| # | 作業 | 詳細 |
|---|---|---|
| 1-1 | `docs/book/`・`docs/ja/book/` を git 管理から除外 | `.gitignore` 追記 + `git rm -r --cached`。約 21MB 削減。`deploy-docs.yml` が CI で mdBook ビルドすることを確認してから実施 |
| 1-2 | `VERERSION` タイポ修正 | 全 8 箇所(辞書クレート 6 + `lindera-dictionary/src/lib.rs:17` + `lindera/src/lib.rs:16`)を `VERSION` に。private const のため非破壊 |
| 1-3 | コメントアウトコードの削除 | `lindera/src/dictionary.rs:217-245` の 6 辞書分のデッドコメントを削除(必要なら git 履歴に残ることを明記) |
| 1-4 | 未使用依存の削除 | 辞書クレート 6 個の `anyhow` / `byteorder` / `csv` を `[dependencies]` から削除(build-dependencies のみ残す)。`cargo udeps` または `cargo machete` で全クレートを横断確認 |
| 1-5 | `bocchan.txt` の重複解消 | 2 重コピーを 1 箇所(`resources/`)に統一し、参照側をパス変更 |
| 1-6 | Cargo.toml の体裁統一 | `lindera-cc-cedict` の余分なスペース等、6 辞書クレートのフォーマット統一 |

- **リスク**: 極小。1-1 のみ docs デプロイフローの確認が必要
- **完了条件**: `cargo build --workspace` / 全テスト / docs デプロイがグリーン
- **規模感**: 小(2〜3 PR)

---

## フェーズ 2: 辞書クレート 6 個の脱コピペ化(挙動変更なし)

**目的**: 約 590 行 × 6 クレートの構造的重複を、宣言的な 1 箇所の定義に集約する。

### 設計方針

`lindera-dictionary` に以下を追加し、各辞書クレートを「パラメータ定義のみ」に縮退させる:

1. **`decl_dictionary!` マクロ(または共通関数)** — `src/embedded.rs` の生成を担う。
   現状の `ipadic_data!` / `EmbeddedIPADICLoader` 等の名前バリエーションは廃し、
   ジェネリックな `EmbeddedLoader`(辞書名をフィールドに持つ)+ マクロで data include を生成
2. **`build.rs` 共通化** — `lindera_dictionary::assets::build_dictionary(FetchParams)` を呼ぶだけの
   3〜5 行に縮退。`FetchParams` は既に共有実装があるため、各クレートの build.rs は定数定義のみ

### 作業項目

| # | 作業 | 詳細 |
|---|---|---|
| 2-1 | `lindera-dictionary` にジェネリック `EmbeddedLoader` 実装 | 現 6 種の `EmbeddedXxxLoader` の共通部を抽出。`include_bytes!` のパスはマクロ引数で注入 |
| 2-2 | 6 クレートの `embedded.rs` をマクロ呼び出しに置換 | 各クレート 88–96 行 → 10 行程度 |
| 2-3 | 6 クレートの `build.rs` を共通関数呼び出しに置換 | jieba の `src_subdir` 差分はパラメータで吸収(既に `FetchParams` がサポート済み) |
| 2-4 | `lindera/src/dictionary.rs` の feature 分岐整理 | 12 個の条件付き import と 3 回反復する `#[cfg(any(...))]` を、辞書レジストリ的なマクロ 1 箇所に集約 |
| 2-5 | 公開名の互換維持 | `EmbeddedIPADICLoader` 等を `pub type` エイリアス + `#[deprecated]` で残す(削除はフェーズ 6) |

- **リスク**: 中。feature フラグの組み合わせ爆発に注意。CI で `embed-*` 各 feature 単体 +
  `embed-cjk` 系のビルドマトリクスを必ず通す
- **完了条件**: 全 feature 組み合わせでビルド・ゴールデンテストがグリーン。
  辞書クレート 1 つあたりの実装が 20 行以下
- **規模感**: 中(3〜4 PR: 共通実装 → 2 クレート移行 → 残り 4 クレート → facade 整理)
- **削減見込み**: 約 500 行 + 将来の辞書追加コストが「定数定義 1 ファイル」に低下

---

## フェーズ 3: コアクレートの内部品質改善(挙動変更なし)

**目的**: `lindera` / `lindera-dictionary` / `lindera-crf` の重複・巨大ファイル・unwrap を解消する。
**前提**: フェーズ 0 のゴールデンテスト・ベンチ基準値が必須。

### 3a. タグフィルタの統合

| # | 作業 | 詳細 |
|---|---|---|
| 3a-1 | 汎用 `tags_filter` 内部モジュール新設 | keep/stop を `FilterPolicy { Keep, Stop }` で、日本語固有の「4 要素へのタグ正規化」を言語別ストラテジで表現 |
| 3a-2 | 4 フィルタを薄いラッパー化 | `JapaneseKeepTagsTokenFilter` 等の公開型・設定フォーマット・挙動は完全維持。実装のみ委譲 |

約 800 行 → 200 行。設定 JSON の互換性はゴールデンテスト + 既存ユニットテスト(112 個)で担保。

### 3b. ローダー層の統一

| # | 作業 | 詳細 |
|---|---|---|
| 3b-1 | `loader/` 5 ファイルの `load`/`load_mmap` パターンをマクロまたはジェネリック関数に集約 | 約 200 行 → 80 行 |
| 3b-2 | 未使用の `DictionaryLoader` トレイト(`loader.rs`)の役割を再定義 | 実際に使う形に直すか削除(削除は非公開なら即、公開ならフェーズ 6) |

### 3c. エラー処理の正常化

| # | 作業 | 詳細 |
|---|---|---|
| 3c-1 | エラー戦略の明文化 | 「公開 API は `LinderaResult`、内部の文脈付与に `anyhow`」を CONTRIBUTING に明記 |
| 3c-2 | `builder.rs` の `.unwrap()` 5 箇所を `?` 化 | `*BuilderOptions::builder()` が `Result` を返す設計自体の見直し含む(非公開なら即、公開ならシグネチャ互換に注意) |
| 3c-3 | `prefix_dictionary.rs` の CSV パース `unwrap()` 13 箇所をエラー化 | 不正な辞書 CSV でパニックせず、行番号付きエラーを返す |
| 3c-4 | `Regex::new().unwrap()` を `LazyLock`(std)化 | `feature_extractor.rs` の 3 箇所 + capture unwrap の防御化 |
| 3c-5 | `assets.rs` の環境変数 `unwrap()` 2 箇所の修正 | |
| 3c-6 | `lindera-crf` の `trainer.rs`(28 箇所)/ `forward_backward.rs`(9 箇所)の unwrap 監査 | 数値変換 `try_from().unwrap()` は不変条件をコメント化するか `expect("理由")` に統一。ホットパスはパフォーマンス維持を優先し、無理にすべて Result 化しない |

### 3d. 巨大ファイルの分割(機械的な移動のみ。ロジック変更禁止)

| 対象 | 分割案 |
|---|---|
| `lindera/src/segmenter.rs`(1,882 行) | `segmenter/mod.rs` + `segmenter/config.rs`(設定パース)+ テストを下位モジュールへ。本体ロジックは 500 行以下に |
| `lindera-dictionary/src/viterbi.rs`(1,147 行) | `viterbi/{lattice.rs, edge.rs, word_entry.rs}` に分割。`Lattice::set_text()`(220 行)を「文字情報バッファ構築 / 辞書スキャン / 格子構築」の 3 関数に分解 |
| `lindera-dictionary/src/trainer/model.rs`(1,248 行) | シリアライズ部とモデル本体を分離 |
| `lindera/src/token_filter/japanese_number.rs`(1,257 行) | 数値正規化ステートマシンとテストの分離 |

- **リスク**: 中〜高(viterbi はホットパス)。3d は `pub use` による再エクスポートでパスを維持し、
  ベンチマークで 3% 以上の劣化がないことを確認
- **完了条件**: ゴールデンテスト・全ユニットテスト・ベンチがグリーン。非テストコードの unwrap が
  「不変条件をコメントで説明した expect」または `?` のみになる
- **規模感**: 大(6〜8 PR)。3a / 3b / 3c / 3d はそれぞれ独立に進行可能

---

## フェーズ 4: バインディング共通レイヤーの導入

**目的**: 5 バインディング × 2,000 行超の重複を共有クレートに集約し、API の不一致を解消する。

### 設計方針

新クレート **`lindera-binding-core`**(FFI 非依存・pure Rust)を新設:

- `TokenizerFacade` — builder 構築 / from_file / set_mode / filter 追加 / tokenize / tokenize_nbest の
  共通フロー。各バインディングは「FFI 型 ⇔ serde_json::Value」の変換だけを実装
- 共通 DTO — `TokenDto` / `SchemaDto` / `MetadataDto`(serde 対応)。各 FFI への変換は
  `From`/`TryFrom` を各バインディング側に薄く実装
- エラーは `BindingError`(`thiserror`)1 種に統一し、各バインディングは自言語の例外型への
  マッピング関数 1 つだけ持つ

各バインディングの値変換ヘルパー(`util.rs` / `convert.rs`、合計約 414 行)は
「FFI 値 → `serde_json::Value`」変換に役割を限定して残す(これは言語固有なので消せない)。

### 作業項目

| # | 作業 | 詳細 |
|---|---|---|
| 4-1 | `lindera-binding-core` 新設 + Python を最初に移行 | 最大のバインディング(2,777 行)で設計を検証 |
| 4-2 | Ruby / PHP / Node.js / WASM を順次移行 | 1 バインディング = 1 PR |
| 4-3 | API 不一致の解消(各言語内で非破壊にできる範囲) | `Token.details` の型統一(PHP を `Option` 相当の null 許容に)、schema/metadata 構築 API の意味的統一。**言語パッケージとして破壊的になるものは各バインディングのメジャーバージョンアップに合わせる** |
| 4-4 | 機能パリティ表の作成と充足 | segmenter / character_filter / token_filter の有無の差(PHP・Node.js に segmenter なし等)を docs に明記し、可能なものは追加実装 |
| 4-5 | WASM のテスト内 unwrap 40+ の整理(任意) | テストヘルパー導入で削減 |

- **リスク**: 中。各バインディングの CI(マルチプラットフォームビルド)が長いため、PR を分けて
  release.yml のビルドジョブを個別に検証
- **完了条件**: 5 バインディングの tokenizer/schema/metadata 実装がそれぞれ 150 行以下の
  FFI 変換層のみになる。横断機能パリティ表が docs に存在
- **規模感**: 大(5〜7 PR)
- **削減見込み**: 約 2,000 行 + コア API 変更時の追従箇所が 5 → 1 に

---

## フェーズ 5: ビルド基盤・CI・CLI の整理(挙動変更なし)

**目的**: Makefile / GitHub Actions / CLI の定型反復を構造化する。

| # | 作業 | 詳細 |
|---|---|---|
| 5-1 | Makefile のループ化 | 13 クレート × 5 種の手書きターゲットを `CRATES` 変数 + パターンルール(`format-%:` 等)に。458 行 → 150 行程度。クレート固有の feature フラグはクレート別変数で表現 |
| 5-2 | CI: composite action 抽出 | checkout / toolchain / cache のセットアップ(20+ ジョブで重複)を `.github/actions/setup-rust/` に |
| 5-3 | CI: テストジョブの共通化 | `release.yml` と `regression.yml` に二重定義された 13 テストジョブを `workflow_call` による再利用可能ワークフロー 1 本に。辞書 6 種テストは matrix 化 |
| 5-4 | CI: バインディングリリースジョブの matrix 統一 | Ruby/PHP/Node.js のプラットフォームマトリクス重複を整理。バージョン検出(`cargo metadata \| jq` × 5 箇所)を 1 ジョブの output 化 |
| 5-5 | CLI リファクタ | ① `tokenize()` 内の出力フォーマット match 二重化(main.rs:430-444 / 447-457 の 28 行)を統合 ② `LinderaErrorKind::Io.with_error(...)` 15 回反復をヘルパー/`From` 実装に ③ 673 行の main.rs をサブコマンド別モジュールに分割 ④ フェーズ 0 で追加した CLI スモークテストで担保 |

- **リスク**: 中(CI は壊すと気づきにくい)。5-3/5-4 はまず regression.yml で検証し、
  リリースフローは dry-run またはタグ打ち前のテストリリースで確認
- **完了条件**: CI 定義の合計行数が半減(約 2,000 行 → 1,000 行程度)、Makefile が 200 行以下、
  regression / release 両ワークフローがグリーン
- **規模感**: 中(3〜4 PR)

---

## フェーズ 6: 公開 API 再設計と破壊的変更の一括実施(v4.0.0)

**目的**: フェーズ 1〜5 で `#[deprecated]` に留めた項目と、semver 上先送りした破壊的変更を
メジャーバージョンで一括清算する。

| # | 作業 | 詳細 |
|---|---|---|
| 6-1 | viterbi 内部のカプセル化 | `WordId` / `WordEntry` / `Edge` / `PathEntry` / `Lattice` の `pub` フィールドをアクセサメソッド化。`WordEntry` のシリアライズ詳細(`SERIALIZED_LEN` 等)を非公開に |
| 6-2 | deprecated 項目の削除 | フェーズ 2 の `EmbeddedXxxLoader` エイリアス、`LINDERA_CACHE` 環境変数シム(`assets.rs:238`)を削除 |
| 6-3 | 旧形式互換コードの削除判断 | ユーザー辞書の「5-bit variant-count encoding」旧形式互換(`viterbi.rs:409,953`)について、対応辞書バイナリの世代を調査の上、削除または明示的なマイグレーションツールに移行 |
| 6-4 | `pub` 監査 | `cargo public-api` 等で公開 API を棚卸しし、実装詳細の露出(builder の `*Options` 命名不統一含む)を整理 |
| 6-5 | ビルダー API の `Result` 化 | フェーズ 3c で内部対応した `*BuilderOptions::builder().unwrap()` パターンの公開シグネチャを是正 |
| 6-6 | マイグレーションガイド作成 | `docs/` に v3 → v4 移行ガイド(日英)を追加 |

- **リスク**: 高(エコシステム影響)。事前に v4.0.0-alpha を切り、主要ユーザー
  (lindera-tantivy 等の連携プロジェクト)で検証期間を設ける
- **完了条件**: `cargo public-api` の差分がマイグレーションガイドと一致。
  全バインディングが v4 コアでグリーン
- **規模感**: 中〜大(4〜6 PR + alpha/beta リリースサイクル)

---

## フェーズ間の依存関係と推奨順序

```
フェーズ0 (安全網) ──┬─→ フェーズ1 (即時クリーンアップ)
                      ├─→ フェーズ2 (辞書クレート統合) ──┐
                      ├─→ フェーズ3 (コア品質) ──────────┤
                      ├─→ フェーズ4 (バインディング統合) ┼─→ フェーズ6 (v4.0.0)
                      └─→ フェーズ5 (ビルド基盤/CI)──────┘
```

- フェーズ 1・2・3・5 は**並行可能**(コンフリクト面: フェーズ 2 と 3 は `lindera-dictionary` を共有するため、2 → 3b の順を推奨)
- フェーズ 4 はフェーズ 3 のコア API 安定後が望ましい(追従コスト削減のため)
- フェーズ 6 のみ全フェーズ完了後

## 効果の見積もり(概算)

| 項目 | Before | After |
|---|---|---|
| 辞書クレートの実装行数 | 約 590 行(6 クレート重複) | 約 100 行(定義のみ) |
| バインディングの重複行数 | 約 2,000 行超 | 各 150 行以下の FFI 変換層 |
| タグフィルタ | 約 800 行 | 約 200 行 |
| Makefile | 458 行 | 約 150 行 |
| CI 定義 | 約 2,028 行 | 約 1,000 行 |
| リポジトリサイズ | docs 生成物 21MB 込み | 21MB 削減 |
| 非テストコードの素の unwrap | 30+ 箇所 | 0(理由付き expect か `?` のみ) |
| 新辞書追加コスト | クレート 1 式コピペ | パラメータ定義 1 ファイル |

## 各 PR 共通の完了チェックリスト

- [ ] `cargo fmt --check` / `cargo clippy --workspace --all-targets`(Makefile の lint 相当)がグリーン
- [ ] 全 feature 組み合わせ(少なくとも `embed-*` 単体 6 通り + `embed-cjk` + `train` + `mmap`)でビルド成功
- [ ] フェーズ 0 のゴールデンテストがグリーン(= トークナイズ結果不変)
- [ ] ベンチマーク劣化 3% 以内(viterbi / segmenter / tokenizer に触れた場合)
- [ ] 公開 API の意図しない変更がない(フェーズ 6 以外)

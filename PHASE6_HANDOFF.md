# Phase 6 引き継ぎ資料 (v4.0.0 破壊的変更)

> このドキュメントは、Lindera のリファクタリング **Phase 6** を別のセッション（IDE の Claude 等）に
> 引き継ぐための **自己完結した** 資料です。前提知識ゼロで読めるように書いています。
> 全体計画は `REFACTORING_PLAN.md`、非破壊フェーズの成果はそちらの「進捗サマリ」を参照してください。

---

## 0. このドキュメントの位置づけ

- **Phase 0〜5（非破壊 / patch・minor）は完了済み**（PR #689〜#706、すべて main にマージ）。
- **Phase 6 は唯一残った破壊的変更（semver major = v4.0.0）のまとめ**。Phase 0〜5 で
  「破壊的だから」と意図的に先送りした項目を、1 つのメジャーバージョンに集約する。
- Phase 6 は **エコシステム影響が大きい**ため、`v4.0.0-alpha` を切って連携プロジェクト
  （`lindera-tantivy`, `lindera-sqlite` 等）で検証する期間を設けることを強く推奨。

---

## 1. リポジトリ構成（最小限の地図）

ワークスペース（`Cargo.toml` の `[workspace]`、`edition = "2024"`, `version = 3.0.7`）:

```
lindera-crf            … CRF 学習(train feature 専用)
lindera-dictionary     … 解析エンジンの中核(viterbi, builder, assets, loader, trainer)
lindera-ipadic / -ipadic-neologd / -unidic / -ko-dic / -cc-cedict / -jieba … 辞書クレート(マクロで定義)
lindera                … 公開ファサード(segmenter, tokenizer, token, token_filter, character_filter)
lindera-cli            … CLI(commands/ にサブコマンド分割済み)
lindera-binding-core   … ★Phase 4 で新設。FFI 非依存の共有ロジック(TokenView, schema)
lindera-python / -php / -ruby / -nodejs / -wasm … 言語バインディング
```

Phase 6 の主戦場は **`lindera-binding-core` / 各バインディング / `lindera-dictionary/src/viterbi.rs`**。

---

## 2. 開発環境のセットアップ（重要・最初に必ず読む）

### 2.1 辞書のダウンロードについて（ハマりどころ）

辞書クレートは `embed-*` feature 有効時、ビルド時に `https://lindera.dev/...` から辞書アーカイブを
ダウンロードする。**ネットワークポリシーで lindera.dev が遮断されている環境では失敗する。**

回避策: 辞書アーカイブは **GitHub ミラーのタグ付きソースアーカイブと MD5 まで完全一致**する。
`LINDERA_DICTIONARIES_PATH` のキャッシュに事前配置すればオフラインでもビルドできる。

```sh
# キャッシュ作成例（バージョンは Cargo.toml の version に合わせる。現状 3.0.7 / v4 では 4.0.0 等）
mkdir -p $HOME/.lindera_cache/<version>
cd $HOME/.lindera_cache/<version>
# IPADIC（他辞書も同様に GitHub の lindera/mecab-* リポジトリのタグから）
curl -sL -o mecab-ipadic-2.7.0-20250920.tar.gz \
  https://github.com/lindera/mecab-ipadic/archive/refs/tags/2.7.0-20250920.tar.gz
curl -sL -o mecab-ko-dic-2.1.1-20180720.tar.gz \
  https://github.com/lindera/mecab-ko-dic/archive/refs/tags/2.1.1-20180720.tar.gz
curl -sL -o mecab-jieba-0.1.1.tar.gz \
  https://github.com/lindera/mecab-jieba/archive/refs/tags/0.1.1.tar.gz
# 各 build.rs の md5_hash と md5sum を照合して一致を確認すること
export LINDERA_DICTIONARIES_PATH=$HOME/.lindera_cache
```

URL とハッシュは各 `lindera-<dict>/build.rs` の `FetchParams`（`file_name` / `download_urls` /
`md5_hash`）を参照。CI 環境では lindera.dev に到達できるのでこの回避は不要。

### 2.2 検証コマンド（毎 PR で必須）

```sh
export LINDERA_DICTIONARIES_PATH=$HOME/.lindera_cache   # オフライン環境のみ

# ゴールデンテスト（トークナイズ結果のスナップショット。挙動変化を機械検出する安全網）
cargo test -p lindera --features embed-ipadic,embed-ko-dic,embed-jieba,train --test golden_tokenization
# lindera 本体 + CLI
cargo test -p lindera --features embed-ipadic,embed-ko-dic,train
cargo test -p lindera-cli --features train,embed-ipadic
# バインディングは FFI ツールチェーン無しでも Rust 部分を検証可能
cargo test -p lindera-python --lib   # nodejs / wasm 等も --lib で可
cargo check -p lindera-php -p lindera-ruby

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

> **Phase 6 は破壊的変更なので、ゴールデンテストの「更新」が正当な場面がある**（出力仕様を
> 意図的に変える場合）。その際は `INSTA_UPDATE=always cargo test ... --test golden_tokenization`
> でスナップショットを更新し、差分を PR で必ずレビューすること。挙動を変えない作業では更新しない。

### 2.3 ベンチ（ホットパスに触れる場合のみ）

`BENCHMARKING.md` 参照。**重要な教訓（Phase 3d）**: `cargo bench` の既定 profile は
`lto=false` で、本番(`[profile.release] lto=true`)と最適化が異なる。ホットパス（viterbi/segmenter）の
性能判断は **必ず本番相当**で行うこと:

```toml
# 計測時だけ一時的に Cargo.toml へ
[profile.bench]
lto = true
codegen-units = 1
```

`--save-baseline` で変更前、`--baseline` で変更後を**同一マシン**で比較し、3% 以内を確認。

---

## 3. これまでの成果（Phase 6 が前提とする状態）

- **安全網**: `lindera/tests/golden_tokenization.rs`（IPADIC/ko-dic/Jieba × Normal/Decompose +
  ユーザー辞書 + N-best、計 8 スナップショット）、`lindera-cli/tests/cli.rs`（8 スモーク）。
- **`lindera-binding-core` 新設済み**（`src/lib.rs`, `token.rs`, `schema.rs`）:
  - `TokenView::from_token(lindera::token::Token)` … 5 バインディングが利用中。
  - `schema::default_dictionary_fields()` / `schema::validate_record()` … Python/PHP/Ruby/Node.js が利用中。
- **未着手で Phase 6 に持ち越した破壊的項目** ＝ 本資料の第 5 章。

---

## 4. Phase 6 のゴールと全体方針

**目標**: 「最も美しい最終形」＝ 各バインディングを **FFI 変換層だけ** に純化し、ロジックを
`lindera-binding-core` のファサードに集約する。あわせて公開 API の不整合・カプセル化漏れを是正する。

**方針**:
1. **すべて 1 つのメジャー（v4.0.0）に集約**。alpha/beta を切り、連携プロジェクトで検証。
2. semver: workspace `version` を `4.0.0` に上げる。各言語パッケージ（PyPI/npm/gem/Packagist）の
   メジャーも合わせる。
3. **破壊的変更は「意図的」**。Phase 0〜5 と違いゴールデン更新もありうるが、**変更は最小限・明示的に**。
4. PR は小さく。各 PR で `REFACTORING_PLAN.md` を更新すると **毎回コンフリクトする**（Phase 0〜5 で
   多発）。**計画書/ドキュメントの更新はコード PR に混ぜず、節目で 1 本にまとめる**こと。

---

## 5. タスク詳細

### 5-1. 完全ファサード化（lindera-binding-core の拡張）

**現状**: 各バインディングの `tokenizer.rs` / `schema.rs` / `metadata.rs` は FFI クラス
（`#[pyclass]` / `#[napi]` / magnus / wasm-bindgen）と密結合で、メソッドの大半が
`inner.method()` への委譲か公開属性。Phase 4 では「非破壊で抜ける純粋ロジック」（token 抽出、
schema 既定/検証）だけを core 化した。

**Phase 6 のやること**: `lindera-binding-core` に以下を追加し、各バインディングを薄いアダプタに:

- `CoreTokenizerBuilder` / `CoreTokenizer` … build フロー（`set_mode` / `set_dictionary` /
  `set_user_dictionary` / `append_*_filter` / `build` / `tokenize` / `tokenize_nbest`）の
  オーケストレーションを集約。各バインディングは「FFI 型 ⇔ `serde_json::Value`」変換と
  `#[pyclass]` 等の薄いラッパーだけを持つ。
- `CoreSchema` / `CoreFieldType` / `CoreFieldDefinition` … schema のフィールド管理・index map・
  `get_field_by_name` 等を集約（Phase 4 は default/validate のみだった）。
- `CoreMetadata` … デフォルト値とスキーマ配線を集約。
- `CoreError` + `ErrorKind` … 共通エラー型。各バインディングは「`CoreError` → 自言語例外」の
  `From` 実装 1 つだけ持つ。

**なぜ破壊的か**: 例えば Python の `schema.fields` は現在 `#[pyo3(get)]` の**属性**。`inner: CoreSchema`
にラップすると getter 化され、利用側の API が変わる（属性 → メソッド/プロパティ）。各言語の
公開 API が微妙に変わるため major でしか出せない。

**値変換は core 化しない**: `serde_json::Value` ⇔ FFI 型（PyObject/Zval/magnus Value/JsValue）は
本質的に FFI 依存。各バインディングの `util.rs`/`convert.rs` に残す（トレイト化に留めるのは可）。

**進め方の推奨**: 最大の `lindera-python`（最も機能が揃っている）で設計を確定 → 残りを順次移行。
各移行は `cargo test -p <binding> --lib` + 各言語テスト（`make test-lindera-<binding>`）で検証。

### 5-2. viterbi 内部構造体のカプセル化

`lindera-dictionary/src/viterbi.rs` の内部構造体が全フィールド `pub` で実装詳細が露出している:

| 構造体 | 公開フィールド（→ アクセサ化 or 非公開化） |
|---|---|
| `WordId` (51) | `id`, `is_system`, `lex_type` |
| `WordEntry` (104) | `word_id`, `word_cost`, `left_id`, `right_id`、`SERIALIZED_LEN`/serialize/deserialize |
| `Edge` (161) | `edge_type`, `word_entry`, `path_cost`, `left_index`, `start_index`, `stop_index`, `kanji_only` |
| `PathEntry` (184) | `edge_index`, `left_pos`, `left_index`, `cost` |
| `Lattice` (196) | 内部バッファ |

**注意（Phase 3d の教訓）**: viterbi は**極めて最適化に敏感**。フィールドをアクセサメソッド化する際、
`#[inline]` を付けないとインライン化が外れて性能回帰する可能性。**本番相当 LTO ベンチ（2.3 節）で
3% 以内を必ず確認**すること。また、`lindera`/`lindera-cli`/bench/テストが
`token.word_id.id` 等で直接フィールドアクセスしているので、アクセサ化に伴い呼び出し側の修正が必要
（grep `word_id.id`, `.is_unknown()` 等で洗い出す）。

**やること**: フィールドを非 `pub` にしてアクセサ（`#[inline] pub fn id(&self) -> u32` 等）を提供。
`WordEntry` のシリアライズ詳細（`SERIALIZED_LEN` 等）は `pub(crate)` に。

### 5-3. API 不整合の解消

Phase 0〜5 で発見し、挙動維持のため温存した不整合:

1. **`Token.details` の型がバインディング間で不一致**:
   - Python / Ruby / Node.js: `Option<Vec<String>>`
   - PHP / WASM: `Vec<String>`
   → どちらかに統一（推奨: `Option` をやめて常に `Vec`、空なら空配列。null 安全）。

2. **デフォルトスキーマのフィールド名不一致**:
   - Python/PHP/Ruby/Node.js の `Schema.create_default()`: `middle_pos` / `small_pos` / `fine_pos`
   - `lindera::dictionary::Schema::default()`（WASM が利用）: `pos_detail_1` / `pos_detail_2` / `pos_detail_3`
   → どちらかに統一。`lindera-binding-core/src/schema.rs::default_dictionary_fields()` の doc コメントに
     経緯あり。コアの `pos_detail_*` に寄せるのが自然だが、既存ユーザーの辞書スキーマ期待に影響するため
     移行ガイドで明示。

3. **ビルダーの戻り値・機能パリティの不一致**:
   - Python はフルーエント（`PyRefMut<Self>`）、他は in-place 変更。
   - `character_filter` / `token_filter` / `segmenter` モジュールの有無が言語ごとにバラバラ
     （PHP・Node.js に segmenter なし等）。→ 完全ファサード化（5-1）に揃えて統一。

### 5-4. 後方互換シム・旧形式の削除

- `lindera-dictionary/src/assets.rs` の **`LINDERA_CACHE` 環境変数（非推奨）** を削除し
  `LINDERA_DICTIONARIES_PATH` に一本化。
- ユーザー辞書の「5-bit variant-count encoding 旧形式互換」（`viterbi.rs` 内、要 grep で現在位置確認）の
  扱いを判断。対応辞書バイナリの世代を調べ、削除可能なら削除、必要ならマイグレーションツール化。
- Phase 2 で互換のため残した `EmbeddedIPADICLoader` 等の旧名エイリアス（もしあれば）を整理。

### 5-5. 移行ガイド

- `docs/`（mdBook、日英両方: `docs/src/` と `docs/ja/src/`）に **v3 → v4 移行ガイド**を追加。
- `cargo public-api` 等で v3↔v4 の公開 API 差分を機械的に洗い出し、ガイドと一致させる。
- 各言語パッケージ（Python/Node.js/Ruby/PHP/WASM）の破壊的変更点（属性→メソッド、`details` 型、
  スキーマ名）をユーザー向けに列挙。

---

## 6. 作業の進め方

1. **開発ブランチ**: 専用のフィーチャーブランチを切る（例 `claude/phase6-v4`）。main へ直接 push しない。
2. **PR 単位**: タスクを小さく分割。例: 「viterbi カプセル化」「CoreSchema 移行(Python)」など 1 PR。
   各 PR で `cargo fmt --check` / `clippy -D warnings` / 関連テストをグリーンに。
3. **CI**: `regression.yml` が PR で走る。バインディングのマルチプラットフォームビルドは時間がかかるため、
   PR を分けて個別に検証。`release.yml` は PR では走らない（タグ/dispatch のみ）。
4. **ネットワーク起因の CI 失敗**: crates.io への curl が `SSL_ERROR_SYSCALL` / `schannel ... close_notify`
   で稀に落ちる。コードと無関係なので**失敗ジョブの再実行**（rerun failed jobs）で対応。
5. **コンフリクト対策**: ドキュメント（`REFACTORING_PLAN.md` 等）の更新をコード PR に混ぜない。
   混ぜると毎回マージ競合する（Phase 0〜5 で多発した）。
6. **alpha リリース**: 主要タスク完了後 `v4.0.0-alpha` を publish し、`lindera-tantivy` /
   `lindera-sqlite` 等で動作確認してから正式 v4.0.0。

---

## 7. 落とし穴・知見（Phase 0〜5 で実証済み）

- **viterbi はモジュール分割しただけで本番 LTO ビルドでも ~5% 回帰する**（実測）。Phase 6 の
  カプセル化でも `#[inline]` + 本番相当ベンチ必須。単純な「きれいにする」変更が性能を壊しうる。
- **ローカル composite action（`uses: ./...`）は `actions/checkout` の後でないと解決できない**。
  CI をいじる場合は checkout を各ジョブに残すこと。
- **辞書アーカイブ = GitHub ミラーのタグ付きソース（MD5 一致）**。オフライン環境での検証はこれで可能。
- **bindings は `cargo check` / `cargo test --lib` が FFI ツールチェーン無しで通る**ので、純粋ロジックの
  検証は plain cargo で完結する（フル FFI ビルドは CI に任せる）。
- **`cargo bench` 既定は非 LTO**。本番性能の判断には `[profile.bench] lto=true` を一時設定。

---

## 8. 着手順の推奨

1. まず **5-2 viterbi カプセル化**（範囲が明確、`lindera-dictionary` 内で完結、ベンチで安全確認）。
2. 次に **5-1 完全ファサード化**を Python から（最大バインディングで設計確定 → 横展開）。
3. 並行して **5-3 API 不整合**は 5-1 の中で吸収（schema/metadata/details/builder）。
4. **5-4 シム削除**は独立して随時。
5. 最後に **5-5 移行ガイド**を `cargo public-api` の差分に基づき作成。
6. 全体を `v4.0.0-alpha` で連携プロジェクト検証 → 正式リリース。

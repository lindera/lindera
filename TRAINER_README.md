# Lindera Trainer - 技術仕様書

Linderaの学習器機能の内部実装と技術詳細についてのドキュメントです。

## 概要

Lindera Trainerは条件付き確率場(CRF)ベースの形態素解析器学習システムです。以下の高度な機能を提供します：

- **CRFベースの統計的学習**: rucrf crateを用いた効率的な実装
- **L1正則化**: オーバーフィッティングを防ぐ正則化機構
- **マルチスレッド対応**: 並列処理による高速な学習
- **包括的Unicode対応**: CJK拡張文字セットの完全サポート
- **高度な未知語処理**: 混合文字種の知的分類アルゴリズム
- **多段階重み最適化**: 学習済み重みの高度な正規化システム
- **Lindera辞書互換**: 既存辞書フォーマットとの完全互換性

## CLIの使用方法

CLIコマンドの詳細な使用方法については [lindera-cli/README.md](./lindera-cli/README.md#train-dictionary) を参照してください。

## 必要なファイルフォーマット仕様

### 1. **語彙辞書 (lex.csv)**

**役割**: 基本語彙辞書
**形式**: MeCab形式のCSV

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

- **用途**: 学習に使う基本的な単語とその品詞情報を定義
- **構成**: `表層形,左文脈ID,右文脈ID,コスト,品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音`

### 2. **未知語定義 (unk.def)**

**役割**: 未知語処理定義
**形式**: 文字種別ごとの未知語パラメータ

```csv
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

- **用途**: 辞書にない単語（未知語）の処理方法を文字種別に定義
- **注意**: これらのラベルは内部処理用で、最終的な辞書ファイルには出力されません

### 3. **学習コーパス (corpus.txt)**

**役割**: 学習データ（正解付きコーパス）
**形式**: タブ区切りの分かち書き

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
EOS
```

- **用途**: 実際の学習に使う文とその正解分析結果
- **形式**: 各行は`表層形\t品詞情報`、文の終わりは`EOS`
- **重要**: 学習の品質はこのコーパスの量と質に大きく依存します

### 4. **文字種定義 (char.def)**

**役割**: 文字種定義
**形式**: 文字カテゴリと文字コード範囲

```text
# 文字カテゴリ定義（カテゴリ名 互換性フラグ 連続フラグ 長さ）
DEFAULT 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# 文字範囲マッピング
0x3041..0x3096 HIRAGANA  # ひらがな
0x30A1..0x30F6 KATAKANA  # カタカナ
0x4E00..0x9FAF KANJI     # 漢字
0x0030..0x0039 NUMERIC   # 数字
0x0041..0x005A ALPHA     # 大文字英字
0x0061..0x007A ALPHA     # 小文字英字
```

- **用途**: どの文字がどのカテゴリに属するかを定義
- **パラメータ**: 互換性、連続性、デフォルト長さなどの設定

### 5. **特徴テンプレート (feature.def)**

**役割**: 特徴テンプレート定義
**形式**: 特徴抽出パターン

```text
# Unigram features (単語レベル特徴)
UNIGRAM:%F[0]         # 品詞（特徴の第0要素）
UNIGRAM:%F[1]         # 品詞細分類1
UNIGRAM:%F[6]         # 原形
UNIGRAM:%F[7]         # 読み（カタカナ）

# Left context features (左文脈特徴)
LEFT:%L[0]            # 左の単語の品詞
LEFT:%L[1]            # 左の単語の品詞細分類

# Right context features (右文脈特徴)
RIGHT:%R[0]           # 右の単語の品詞
RIGHT:%R[1]           # 右の単語の品詞細分類

# Bigram features (組み合わせ特徴)
UNIGRAM:%F[0]/%F[1]   # 品詞 + 品詞細分類
UNIGRAM:%F[0]/%F[6]   # 品詞 + 原形
```

- **用途**: どの情報から学習特徴を作るかを定義
- **テンプレート**: `%F[n]` (特徴), `%L[n]` (左文脈), `%R[n]` (右文脈)

### 6. **特徴正規化ルール (rewrite.def)**

**役割**: 特徴正規化ルール
**形式**: 置換ルール（タブ区切り）

```text
# 数値表現の正規化
数	NUM
*	UNK

# 固有名詞の正規化
名詞,固有名詞	名詞,一般

# 助動詞の簡略化
助動詞,*,*,*,特殊・デス	助動詞
助動詞,*,*,*,特殊・ダ	助動詞
```

- **用途**: 特徴を正規化して学習効率を向上
- **形式**: `元パターン\t置換パターン`
- **効果**: 稀な特徴を一般化してスパースネス問題を軽減

### 7. **出力モデル形式**

**役割**: 出力モデルファイル
**形式**: bincode形式（バイナリ）が標準、JSON形式もサポート

モデルには以下の情報が含まれます：

```json
{
  "feature_weights": [0.0, 0.084, 0.091, ...],
  "labels": ["外国", "人", "参政", "権", ...],
  "pos_info": ["名詞,一般,*,*,*,*,*,*,*", "名詞,接尾,一般,*,*,*,*,*,*", ...],
  "feature_templates": ["UNIGRAM:%F[0]", ...],
  "metadata": {
    "version": "1.0.0",
    "regularization": 0.01,
    "iterations": 100,
    "feature_count": 13,
    "label_count": 19
  }
}
```

- **用途**: 学習結果を保存し、後で辞書生成に使用

## 学習パラメータ仕様

- **正則化係数 (lambda)**: L1正則化の強さを制御（デフォルト: 0.01）
- **最大イテレーション数 (iter)**: 学習の最大反復回数（デフォルト: 100）
- **並列スレッド数 (threads)**: 並列処理のスレッド数（デフォルト: 1）

## API使用例

```rust,no_run
use std::fs::File;
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// 設定ファイルから設定を読み込み
let seed_file = File::open("resources/training/seed.csv")?;
let char_file = File::open("resources/training/char.def")?;
let unk_file = File::open("resources/training/unk.def")?;
let feature_file = File::open("resources/training/feature.def")?;
let rewrite_file = File::open("resources/training/rewrite.def")?;

let config = TrainerConfig::from_readers(
    seed_file,
    char_file,
    unk_file,
    feature_file,
    rewrite_file
)?;

// トレーナーの初期化と設定
let trainer = Trainer::new(config)?
    .regularization_cost(0.01)
    .max_iter(100)
    .num_threads(4);

// コーパスの読み込み
let corpus_file = File::open("resources/training/corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// 学習の実行
let model = trainer.train(corpus)?;

// モデルの保存（バイナリ形式）
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Lindera辞書形式で出力
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

## 実装状況

### 完了済み機能（2024年9月版）

#### **基本機能**

- **基本アーキテクチャ**: 完全なtrainerモジュール構造
- **CRF学習**: rucrf統合によるCondition Random Field学習
- **CLI統合**: `lindera train`コマンドで全パラメータ対応
- **コーパス処理**: MeCab形式コーパスの完全サポート
- **辞書統合**: lex.csv、char.def、unk.defからの辞書構築
- **特徴抽出**: unigram/bigram特徴の抽出と変換
- **モデル保存**: JSON・bincode形式での学習済みモデル出力
- **辞書出力**: Lindera形式辞書ファイル生成

#### **高度な未知語処理**

- **包括的Unicode対応**: CJK拡張、カタカナ拡張、ひらがな拡張の完全サポート
- **カテゴリ別品詞設定**: 文字種に応じた適切な品詞情報の自動割り当て
  - DEFAULT: 名詞,一般（文字種不明）
  - HIRAGANA/KATAKANA/KANJI: 名詞,一般（日本語文字）
  - ALPHA: 名詞,固有名詞（アルファベット）
  - NUMERIC: 名詞,数（数字）
- **表層形解析**: 文字パターン、長さ、位置情報による特徴生成
- **動的コスト計算**: 文字種別・文脈考慮の適応的コスト

#### **リファクタリング済み実装（2024年9月最新版）**

- **定数管理**: cost_constantsモジュールによるマジックナンバー削除
- **メソッド分割**: 大きなメソッドの分割による可読性向上
  - `train()` → `build_lattices_from_corpus()`, `extract_labels()`, `train_crf_model()`, `create_final_model()`
- **コスト計算統一**: 重複コードの統一による保守性向上
  - `calculate_known_word_cost()`: 既知語コスト計算
  - `calculate_unknown_word_cost()`: 未知語コスト計算
- **デバッグ出力整理**: log_debug!マクロによる構造化ログ
- **エラーハンドリング強化**: 包括的なエラー処理とドキュメント

## アーキテクチャ

```text
lindera-dictionary/src/trainer.rs  # メインのTrainer構造体
lindera-dictionary/src/trainer/
├── config.rs           # 設定管理
├── corpus.rs           # コーパス処理
├── feature_extractor.rs # 特徴抽出
├── feature_rewriter.rs  # 特徴リライト
└── model.rs            # 学習済みモデル
```

## 高度な未知語処理システム

### 包括的Unicode文字種判定

最新実装では、基本的なUnicode範囲を大幅に拡張し、以下の文字セットを完全サポートしています：

#### **カテゴリ別品詞設定**

未知語カテゴリごとに適切な品詞情報を自動設定：

```rust
let unk_feature = match *category {
    "DEFAULT" => "名詞,一般,*,*,*,*,*,*,*",      // 文字種不明
    "HIRAGANA" => "名詞,一般,*,*,*,*,*,*,*",     // ひらがな
    "KATAKANA" => "名詞,一般,*,*,*,*,*,*,*",     // カタカナ
    "KANJI" => "名詞,一般,*,*,*,*,*,*,*",        // 漢字
    "ALPHA" => "名詞,固有名詞,*,*,*,*,*,*,*",    // アルファベット（固有名詞として扱う）
    "NUMERIC" => "名詞,数,*,*,*,*,*,*,*",        // 数字（数詞として扱う）
    _ => "名詞,一般,*,*,*,*,*,*,*",
}.to_string();
```

### 特徴重み最適化

#### **コスト計算定数**

```rust
mod cost_constants {
    // 既知語コスト計算
    pub const KNOWN_WORD_BASE_COST: i16 = 1000;
    pub const KNOWN_WORD_COST_MULTIPLIER: f64 = 500.0;
    pub const KNOWN_WORD_COST_MIN: i16 = 500;
    pub const KNOWN_WORD_COST_MAX: i16 = 3000;
    pub const KNOWN_WORD_DEFAULT_COST: i16 = 1500;

    // 未知語コスト計算
    pub const UNK_BASE_COST: i32 = 3000;
    pub const UNK_COST_MULTIPLIER: f64 = 500.0;
    pub const UNK_COST_MIN: i32 = 2500;
    pub const UNK_COST_MAX: i32 = 4500;

    // カテゴリ別調整
    pub const UNK_DEFAULT_ADJUSTMENT: i32 = 0;     // DEFAULT
    pub const UNK_HIRAGANA_ADJUSTMENT: i32 = 200;  // HIRAGANA - 軽微なペナルティ
    pub const UNK_KATAKANA_ADJUSTMENT: i32 = 0;    // KATAKANA - 中程度
    pub const UNK_KANJI_ADJUSTMENT: i32 = 400;     // KANJI - 高いペナルティ
    pub const UNK_ALPHA_ADJUSTMENT: i32 = 100;     // ALPHA - 軽度のペナルティ
    pub const UNK_NUMERIC_ADJUSTMENT: i32 = -100;  // NUMERIC - ボーナス（規則的なため）
}
```

#### **統一されたコスト計算**

```rust
// 既知語コスト計算
fn calculate_known_word_cost(&self, feature_weight: f64) -> i16 {
    let scaled_weight = (feature_weight * cost_constants::KNOWN_WORD_COST_MULTIPLIER) as i32;
    let final_cost = cost_constants::KNOWN_WORD_BASE_COST as i32 + scaled_weight;
    final_cost.clamp(
        cost_constants::KNOWN_WORD_COST_MIN as i32,
        cost_constants::KNOWN_WORD_COST_MAX as i32
    ) as i16
}

// 未知語コスト計算
fn calculate_unknown_word_cost(&self, feature_weight: f64, category: usize) -> i32 {
    let base_cost = cost_constants::UNK_BASE_COST;
    let category_adjustment = match category {
        0 => cost_constants::UNK_DEFAULT_ADJUSTMENT,
        1 => cost_constants::UNK_HIRAGANA_ADJUSTMENT,
        2 => cost_constants::UNK_KATAKANA_ADJUSTMENT,
        3 => cost_constants::UNK_KANJI_ADJUSTMENT,
        4 => cost_constants::UNK_ALPHA_ADJUSTMENT,
        5 => cost_constants::UNK_NUMERIC_ADJUSTMENT,
        _ => 0,
    };
    let scaled_weight = (feature_weight * cost_constants::UNK_COST_MULTIPLIER) as i32;
    let final_cost = base_cost + category_adjustment + scaled_weight;
    final_cost.clamp(
        cost_constants::UNK_COST_MIN,
        cost_constants::UNK_COST_MAX
    )
}
```

## コード品質管理

### **Lint対応**

- **Dead Code**: 未使用メソッドに`#[allow(dead_code)]`適用
- **Format Args**: インライン形式への統一（`"value = {value}"`）
- **Needless Borrows**: 不要な借用の除去

### **テスト品質**

```bash
$ cargo test --features=train
running 41 tests
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo clippy --features=train -- -D warnings
    Finished dev [optimized + debuginfo] target(s) in 2.01s
```

## パフォーマンス最適化

### メモリ効率化

- **遅延評価**: 必要時のみmerged_modelを作成
- **未使用特徴除去**: 学習後の不要特徴自動削除
- **効率的なバイナリ形式**: bincode使用による高速シリアライゼーション

### 並列処理対応

```rust
let trainer = rucrf::Trainer::new()
    .regularization(rucrf::Regularization::L1, regularization_cost)?
    .max_iter(max_iter)?
    .n_threads(self.num_threads)?;  // マルチスレッド学習
```

## 実用的な学習データ要件

### 推奨コーパス仕様

実際のアプリケーションで有効な辞書を生成するための推奨事項：

1. **コーパスサイズ**
   - **最小**: 100文（基本動作確認用）
   - **推奨**: 1,000文以上（実用レベル）
   - **理想**: 10,000文以上（商用品質）

2. **語彙の多様性**
   - 異なる品詞の均等な分布
   - 活用形・語尾変化の網羅
   - 専門用語・固有名詞の適切な含有

3. **品質管理**
   - 人手による形態素解析結果の検証
   - 一貫した分析基準の適用
   - エラー率5%以下の維持

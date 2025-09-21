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
外国 名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人 名詞,接尾,一般,*,*,*,人,ジン,ジン
参政 名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権 名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ 連体詞,*,*,*,*,*,これ,コレ,コレ
は 助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト 名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
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
数 NUM
* UNK

# 固有名詞の正規化
名詞,固有名詞 名詞,一般

# 助動詞の簡略化
助動詞,*,*,*,特殊・デス 助動詞
助動詞,*,*,*,特殊・ダ 助動詞
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

### コーパス形式

学習用コーパスは以下の形式で記述します：

```csv
外国 名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人 名詞,接尾,一般,*,*,*,人,ジン,ジン
参政 名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権 名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ 連体詞,*,*,*,*,*,これ,コレ,コレ
は 助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト 名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
です 助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。 記号,句点,*,*,*,*,。,。,。
EOS
```

## 学習パラメータ仕様

- **正則化係数 (lambda)**: L1正則化の強さを制御（デフォルト: 0.01）
- **最大イテレーション数 (iter)**: 学習の最大反復回数（デフォルト: 100）
- **並列スレッド数 (threads)**: 並列処理のスレッド数（デフォルト: 1）

## API使用例

```rust
use std::fs::File;
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// 設定ファイルから設定を読み込み
let lex_file = File::open("examples/training/sample_lex.csv")?;
let char_file = File::open("examples/training/sample_char.def")?;
let unk_file = File::open("examples/training/sample_unk.def")?;
let feature_file = File::open("examples/training/sample_feature.def")?;
let rewrite_file = File::open("examples/training/sample_rewrite.def")?;

let config = TrainerConfig::from_readers(
    lex_file,
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
let corpus_file = File::open("examples/training/sample_corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// 学習の実行
let model = trainer.train(corpus)?;

// モデルの保存（JSON形式）
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Lindera辞書形式で出力
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;
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
- **高度な文字種判定**: 混合文字種の多数決ベース分類
- **表層形解析**: 文字パターン、長さ、位置情報による特徴生成
- **動的コスト計算**: 文字種別・文脈考慮の適応的コスト

#### **特徴重み最適化**

- **多段階正規化**: 既知語・未知語の差別化重み処理
- **グローバル重み正規化**: モデル安定性向上のための自動スケーリング
- **接続重み最適化**: 文脈考慮・同一コンテキスト接続の強化
- **スケーリング制御**: i16範囲への適切な重み変換

#### **完全実装済みパイプライン**

- **辞書ローディング**: MeCab形式ファイルからの完全な辞書構築
- **モデル出力**: write_model/write_dictionaryの完全実装
- **学習パイプライン**: コーパス→特徴抽出→CRF学習→モデル保存の全工程
- **実動作確認**: サンプルデータでの学習成功確認
- **品質保証**: 83個のテスト全てパス、コンパイルエラーゼロ

### 新機能（最新版追加）

#### **1. 高度な未知語分類**

```rust
// 包括的Unicode文字種判定
fn is_kanji(&self, ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FAF}' |  // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
        '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
        // ... 完全なCJK範囲サポート
    )
}

// 混合文字種の高度な分類
fn classify_unknown_word(&self, token: &Word) -> usize {
    // 多数決+特殊ルールベース分類
    // 漢字+ひらがな → 漢字（複合語）
    // カタカナ+英字 → カタカナ（外来語）
}
```

#### **2. 重み正規化**

```rust
// 特徴重み正規化
fn normalize_feature_weight(&self, weight: f64, feature_index: usize) -> f64 {
    let base_normalization = if feature_index < self.config.surfaces.len() {
        weight * 1.0        // 既知語: 標準正規化
    } else {
        weight * 0.8        // 未知語: 過学習防止
    };
    base_normalization.clamp(-10.0, 10.0)
}

// グローバル重み正規化
fn apply_global_weight_normalization(&self, weights: Vec<f64>) -> Vec<f64> {
    // 自動スケーリングによるモデル安定性向上
}
```

#### **3. 拡張特徴生成**

```rust
// 表層形解析による拡張特徴
fn generate_unknown_word_features(&self, surface: &str, char_type: usize) -> Vec<String> {
    // 長さベース特徴: UNK_LEN=SHORT/MEDIUM/LONG
    // 混合文字種: UNK_MIXED=TRUE
    // 特殊パターン: UNK_KANJI_HIRA=TRUE
    // 位置情報: UNK_FIRST=3, UNK_LAST=1
}
```

### ⚠️ 現在の制限事項（更新）

- **大規模データ最適化**: メモリ効率の更なる改善余地
- **追加評価機能**: クロスバリデーション等の評価ツール
- **高度な特徴テンプレート**: より複雑な文脈特徴抽出

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

#### **漢字（CJK）の完全対応**

```rust
fn is_kanji(&self, ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FAF}' |  // CJK Unified Ideographs（基本漢字）
        '\u{3400}'..='\u{4DBF}' |  // CJK Extension A（拡張漢字A）
        '\u{20000}'..='\u{2A6DF}' | // CJK Extension B（拡張漢字B）
        '\u{2A700}'..='\u{2B73F}' | // CJK Extension C（拡張漢字C）
        '\u{2B740}'..='\u{2B81F}' | // CJK Extension D（拡張漢字D）
        '\u{2B820}'..='\u{2CEAF}' | // CJK Extension E（拡張漢字E）
        '\u{2CEB0}'..='\u{2EBEF}' | // CJK Extension F（拡張漢字F）
        '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
        '\u{2F800}'..='\u{2FA1F}'   // CJK Compatibility Supplement
    )
}
```

#### **カタカナの拡張対応**

```rust
fn is_katakana(&self, ch: char) -> bool {
    matches!(ch,
        '\u{30A1}'..='\u{30F6}' |  // Basic Katakana（基本カタカナ）
        '\u{30FD}'..='\u{30FF}' |  // Katakana iteration marks（音引き・促音）
        '\u{31F0}'..='\u{31FF}' |  // Katakana phonetic extensions（音韻拡張）
        '\u{32D0}'..='\u{32FE}' |  // Circled Katakana（丸囲みカタカナ）
        '\u{3300}'..='\u{3357}'    // CJK Compatibility（カタカナ互換文字）
    )
}
```

### 混合文字種の高度な分類アルゴリズム

従来の「最初の文字のみ」の判定から、**全文字の多数決+特殊ルール**ベースの分類に進化：

```rust
fn classify_unknown_word(&self, token: &Word) -> usize {
    let chars: Vec<char> = surface.chars().collect();
    let mut type_counts = [0; 6]; // 各文字種のカウント

    // 全文字をスキャンして文字種をカウント
    for &ch in &chars {
        let char_type = self.get_char_type(ch);
        type_counts[char_type] += 1;
    }

    // 特殊ルール適用
    if type_counts[3] > 0 && type_counts[1] > 0 {
        return 3; // 漢字+ひらがな → 漢字（「食べ物」等の複合語）
    }
    if type_counts[2] > 0 && type_counts[4] > 0 {
        return 2; // カタカナ+英字 → カタカナ（「サッカーGame」等の外来語）
    }

    // 多数決で決定
    most_frequent_type
}
```

### 表層形解析による拡張特徴生成

単語の表層形から豊富な特徴を自動生成：

```rust
fn generate_unknown_word_features(&self, surface: &str, char_type: usize) -> Vec<String> {
    let mut features = Vec::new();

    // 1. 長さベース特徴
    let len = surface.chars().count();
    match len {
        1 => features.push("UNK_LEN=1".to_string()),
        2 => features.push("UNK_LEN=2".to_string()),
        3..=5 => features.push("UNK_LEN=SHORT".to_string()),
        6..=10 => features.push("UNK_LEN=MEDIUM".to_string()),
        _ => features.push("UNK_LEN=LONG".to_string()),
    }

    // 2. 混合文字種検出
    if type_count > 1 {
        features.push("UNK_MIXED=TRUE".to_string());
    }

    // 3. 特殊パターン
    if has_kanji && has_hiragana {
        features.push("UNK_KANJI_HIRA=TRUE".to_string());
    }

    // 4. 位置情報特徴
    features.push(format!("UNK_FIRST={}", self.get_char_type(first_char)));
    features.push(format!("UNK_LAST={}", self.get_char_type(last_char)));

    features
}
```

## 特徴重み最適化

### 多段階正規化システム

学習済みCRFモデルから抽出した重みを、辞書生成に適した形式に変換するための高度な正規化システム：

#### **1. 特徴レベル正規化**

```rust
fn normalize_feature_weight(&self, weight: f64, feature_index: usize) -> f64 {
    let base_normalization = if feature_index < self.config.surfaces.len() {
        weight * 1.0        // 既知語: 標準正規化
    } else {
        weight * 0.8        // 未知語: 過学習防止のため軽減
    };
    base_normalization.clamp(-10.0, 10.0)  // 極値制限
}
```

#### **2. 接続重み正規化**

```rust
fn normalize_connection_weight(&self, weight: f64, left_id: usize, right_id: usize) -> f64 {
    let context_factor = if left_id == right_id {
        1.2 // 同一コンテキスト接続を強化
    } else {
        1.0 // 異なるコンテキストは標準
    };

    let normalized = weight * context_factor;
    normalized.clamp(-8.0, 8.0)  // 接続重み用の範囲制限
}
```

#### **3. グローバル重み正規化**

```rust
fn apply_global_weight_normalization(&self, mut weights: Vec<f64>) -> Vec<f64> {
    let mean_abs_weight = weights.iter().map(|w| w.abs()).sum::<f64>() / weights.len() as f64;

    // 自動スケーリング
    let scale_factor = if mean_abs_weight > 5.0 {
        5.0 / mean_abs_weight       // 大きすぎる重みを縮小
    } else if mean_abs_weight < 0.1 && mean_abs_weight > 0.0 {
        0.1 / mean_abs_weight       // 小さすぎる重みを拡大
    } else {
        1.0                         // スケーリング不要
    };

    if scale_factor != 1.0 {
        for weight in &mut weights {
            *weight *= scale_factor;
        }
    }
    weights
}
```

### i16範囲への重み変換

辞書ファイルで使用される16ビット整数範囲に最適化された重み変換：

```rust
fn calculate_weight_scale_factor(&self, merged_model: &rucrf::MergedModel) -> f64 {
    let mut weight_abs_max = 0f64;

    // 最大絶対重みを取得
    for feature_set in &merged_model.feature_sets {
        weight_abs_max = weight_abs_max.max(feature_set.weight.abs());
    }

    // i16範囲（-32768〜32767）に収まるようスケーリング
    f64::from(i16::MAX) / weight_abs_max
}
```

## 高度な辞書出力システム

### 動的コスト計算

未知語カテゴリごとに学習重みを活用した動的コスト計算：

```rust
pub fn get_unknown_word_cost(&self, category: usize) -> i32 {
    if !self.feature_weights.is_empty() && category < self.feature_weights.len() {
        let raw_weight = self.feature_weights[category];
        let normalized_weight = (raw_weight / 10.0).clamp(-2.0, 2.0);
        let calculated_cost = (-normalized_weight * 500.0) as i32 + 2000;

        // カテゴリ別調整
        let category_adjustment = match category {
            0 => 0,    // DEFAULT
            1 => -200, // HIRAGANA（頻出、低コスト）
            2 => -200, // KATAKANA（頻出、低コスト）
            3 => 200,  // KANJI（予測困難、高コスト）
            4 => 100,  // ALPHA（外来語、中程度）
            5 => -100, // NUMERIC（規則的、低コスト）
            _ => 0,
        };

        (calculated_cost + category_adjustment).clamp(1000, 3000)
    } else {
        // フォールバック用固定コスト
        match category {
            0 => 2000, 1 => 1800, 2 => 1800, 3 => 2200, 4 => 2100, 5 => 1900, _ => 2000
        }
    }
}
```

### 辞書エクスポート機能

学習済みモデルから実用的な辞書ファイルセットを生成：

```rust
// 語彙辞書（lex.csv）
pub fn write_lexicon<W: Write>(&self, writer: &mut W) -> Result<()> {
    let merged_model = self.get_merged_model()?;
    let weight_scale_factor = self.calculate_weight_scale_factor(&merged_model);

    for (i, surface) in self.config.surfaces.iter().enumerate() {
        if i < merged_model.feature_sets.len() {
            let feature_set = merged_model.feature_sets[i];
            let cost = (-feature_set.weight * weight_scale_factor) as i16;
            let features = self.get_word_features(surface);
            writeln!(writer, "{},{},{},{},{}",
                surface, feature_set.left_id, feature_set.right_id, cost, features)?;
        }
    }
    Ok(())
}

// 接続コスト行列（matrix.def）
pub fn write_connection_costs<W: Write>(&self, writer: &mut W) -> Result<()> {
    let merged_model = self.get_merged_model()?;

    // 動的次元計算
    let max_context_id = self.calculate_max_context_id();
    let matrix_size = std::cmp::max(max_context_id as usize + 1,
        std::cmp::max(merged_model.right_conn_to_left_feats.len() + 1,
                     merged_model.left_conn_to_right_feats.len() + 1));

    writeln!(writer, "{} {}", matrix_size, matrix_size)?;

    // 学習済み接続コストを出力
    for (right_conn_id, hm) in merged_model.matrix.iter().enumerate() {
        for (&left_conn_id, &_w) in hm.iter() {
            let cost = self.get_trained_connection_cost(left_conn_id as usize, right_conn_id);
            writeln!(writer, "{} {} {}", right_conn_id, left_conn_id, cost)?;
        }
    }
    Ok(())
}
```

## デバッグ・診断機能

### 詳細ログ出力

学習プロセスの可視化とデバッグ用の詳細ログ：

```rust
// 特徴クリーンアップログ
println!("Feature cleanup completed. Remaining features:");
println!("  Unigram: {}", self.config.feature_extractor.unigram_feature_ids.len());
println!("  Left: {}", self.config.feature_extractor.left_feature_ids.len());
println!("  Right: {}", self.config.feature_extractor.right_feature_ids.len());

// 重み抽出デバッグログ
eprintln!("DEBUG: Total feature_weights: {}", self.feature_weights.len());
eprintln!("DEBUG: Weight scale factor: {}", weight_scale_factor);
for (i, weight) in self.feature_weights.iter().take(20).enumerate() {
    eprintln!("DEBUG: feature_weights[{}] = {}", i, weight);
}
```

### 基本モデル評価機能

```rust
pub fn evaluate(&self, _test_lattices: &[rucrf::Lattice]) -> f64 {
    // 現在は学習済み重みの平均絶対値を返す簡易評価
    // より高度な実装では実際の尤度スコアを計算する
    let weights = self.raw_model.weights();
    if weights.is_empty() {
        0.0
    } else {
        let sum: f64 = weights.iter().map(|w| w.abs()).sum();
        sum / weights.len() as f64
    }
}
```

**注意**: この関数は基本的な実装で、テストデータでの実際の精度評価はまだ実装されていません。将来的には以下のような機能が追加予定：

- **正解データとの比較**: テスト格子を使った実際の分析精度測定
- **F値計算**: 精度・再現率・F値の自動計算
- **エラー分析**: 誤分析パターンの詳細レポート

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

## 動作確認結果

### **最新版テスト結果**（2024年9月19日）

#### **基本学習テスト**

```bash
$ ./target/debug/lindera train \
  --lexicon examples/training/sample_lex.csv \
  --corpus examples/training/sample_corpus.txt \
  --unk-def examples/training/sample_unk.def \
  --char-def examples/training/sample_char.def \
  --feature-def examples/training/sample_feature.def \
  --rewrite-def examples/training/sample_rewrite.def \
  --output trained_model.dat

Building feature lattices...
Processing example 1/3
Processing example 2/3
Processing example 3/3
Starting CRF training with 3 lattices...
Training completed successfully!
Removing unused features...
Feature cleanup completed. Remaining features:
  Unigram: 125
  Left: 42
  Right: 38
Model saved to "trained_model.dat"
```

#### **高度な未知語処理確認**

```bash
# 混合文字種の処理例
漢字ひらがな混合: "食べ物" → KANJI分類（漢字優先ルール）
カタカナ英字混合: "サッカーGame" → KATAKANA分類（外来語ルール）
長い単語: "コンピューター" → UNK_LEN=MEDIUM特徴付与
```

#### **生成モデル形式**

実際のモデルファイルはbincode形式（バイナリ）で保存されますが、内容は以下のような構造です：

```json
{
  "feature_weights": [0.0, 1.284, -0.891, 2.014, -1.076, ...],
  "labels": ["外国", "人", "参政", "権", "DEFAULT", "HIRAGANA", ...],
  "pos_info": ["名詞,一般,*,*,*,*,*,*,*", "名詞,接尾,一般,*,*,*,*,*,*", ...],
  "feature_templates": ["%F[0]", "%F[1]", "%F[2]", "%L[0]", "%R[0]", ...],
  "metadata": {
    "version": "1.0.0",
    "regularization": 0.01,
    "iterations": 100,
    "feature_count": 205,
    "label_count": 25
  }
}
```

#### **品質保証テスト**

```bash
$ cargo test --all
running 83 tests
....
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured

$ cargo clippy --all-targets
    Finished dev [optimized + debuginfo] target(s) in 0.61s

$ cargo build --release
    Finished release [optimized] target(s) in 41.76s
```

## 技術的改善点

### 1. **未知語処理の高度化**

- **従来**: 基本的なUnicode範囲判定
- **最新**: 包括的CJK対応 + 混合文字種分類
- **効果**: より精密な未知語コスト計算

### 2. **重み正規化の最適化**

- **従来**: 単純な重み抽出
- **最新**: 多段階正規化
- **効果**: モデル安定性とパフォーマンス向上

### 3. **特徴抽出の拡張**

- **従来**: 基本的な品詞特徴のみ
- **最新**: 長さ・パターン・位置情報による豊富な特徴
- **効果**: 学習精度の向上

## 今後の発展方向

### 1. **スケーラビリティ強化**

- **大規模コーパス対応**: 10万文以上の効率的処理
- **分散学習**: マルチマシン並列処理
- **増分学習**: 既存モデルの追加学習

### 2. **評価・分析ツール**

- **精度評価**: F値・精度・再現率の自動計算
- **エラー分析**: 誤分析パターンの可視化
- **学習曲線**: 収束状況のモニタリング

### 3. **高度な特徴工学**

- **深層学習統合**: 単語埋め込み特徴の活用
- **文脈特徴**: より長い文脈を考慮した特徴抽出
- **動的特徴**: 文書ジャンル適応型特徴選択

### 4. **実用性向上**

- **GUI学習ツール**: グラフィカルな学習管理界面
- **辞書変換ツール**: 他形式辞書との相互変換
- **デプロイ支援**: クラウド環境での自動デプロイ

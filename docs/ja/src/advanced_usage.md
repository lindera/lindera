# 高度な使い方

## ユーザー辞書を使用したトークナイズ

デフォルトのシステム辞書に加えて、ユーザー辞書のエントリーを指定することができます。ユーザー辞書は以下のフォーマットのCSVである必要があります。

```csv
<surface>,<part_of_speech>,<reading>
```

Cargo.tomlに以下を追加してください：

```toml
[dependencies]
lindera = { version = "2.1.1", features = ["embed-ipadic"] }
```

例：

```shell
% cat ./resources/user_dict/ipadic_simple_userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

ユーザー辞書を使用する場合、`Tokenizer` は以下のように作成します：

```rust
use std::fs::File;
use std::path::PathBuf;

use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("user_dict")
        .join("ipadic_simple_userdic.csv");

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ipadic")
        .join("metadata.json");
    let metadata: Metadata = serde_json::from_reader(
        File::open(metadata_file)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
            .unwrap(),
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
    .unwrap();

    let dictionary = load_dictionary("embedded://ipadic")?;
    let user_dictionary = load_user_dictionary(user_dict_path.to_str().unwrap(), &metadata)?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        Some(user_dictionary), // 読み込んだユーザー辞書を使用
    );

    // トークナイザーを作成
    let tokenizer = Tokenizer::new(segmenter);

    // テキストをトークナイズ
    let text = "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です";
    let mut tokens = tokenizer.tokenize(text)?;

    // テキストとトークンを表示
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

上記の例は以下のように実行できます：

```shell
% cargo run --features=embed-ipadic --example=tokenize_with_user_dict
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

## フィルタを使用したトークナイズ

Cargo.tomlに以下を追加してください：

```toml
[dependencies]
lindera = { version = "2.1.1", features = ["embed-ipadic"] }
```

この例では、Lindera解析フレームワークの基本的な使い方を説明します。

以下の処理を行います：

- Unicode正規化(NFKC)のための文字フィルタを適用
- IPADICで入力テキストをトークナイズ
- ストップタグ（品詞）の除去と日本語カタカナ語幹フィルタのためのトークンフィルタを適用

```rust
use lindera::character_filter::BoxCharacterFilter;
use lindera::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
use lindera::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::token_filter::BoxTokenFilter;
use lindera::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
use lindera::token_filter::japanese_number::JapaneseNumberTokenFilter;
use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        None, // この例ではユーザー辞書は使用しません
    );

    let unicode_normalize_char_filter =
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

    let japanese_iteration_mark_char_filter =
        JapaneseIterationMarkCharacterFilter::new(true, true);

    let japanese_compound_word_token_filter = JapaneseCompoundWordTokenFilter::new(
        vec!["名詞,数".to_string(), "名詞,接尾,助数詞".to_string()]
            .into_iter()
            .collect(),
        Some("複合語".to_string()),
    );

    let japanese_number_token_filter =
        JapaneseNumberTokenFilter::new(Some(vec!["名詞,数".to_string()].into_iter().collect()));

    let japanese_stop_tags_token_filter = JapaneseStopTagsTokenFilter::new(
        vec![
            "接続詞".to_string(),
            "助詞".to_string(),
            "助詞,格助詞".to_string(),
            "助詞,格助詞,一般".to_string(),
            "助詞,格助詞,引用".to_string(),
            "助詞,格助詞,連語".to_string(),
            "助詞,係助詞".to_string(),
            "助詞,副助詞".to_string(),
            "助詞,間投助詞".to_string(),
            "助詞,並立助詞".to_string(),
            "助詞,終助詞".to_string(),
            "助詞,副助詞／並立助詞／終助詞".to_string(),
            "助詞,連体化".to_string(),
            "助詞,副詞化".to_string(),
            "助詞,特殊".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
            "記号,一般".to_string(),
            "記号,読点".to_string(),
            "記号,句点".to_string(),
            "記号,空白".to_string(),
            "記号,括弧閉".to_string(),
            "その他,間投".to_string(),
            "フィラー".to_string(),
            "非言語音".to_string(),
        ]
        .into_iter()
        .collect(),
    );

    // トークナイザーを作成
    let mut tokenizer = Tokenizer::new(segmenter);

    tokenizer
        .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
        .append_character_filter(BoxCharacterFilter::from(
            japanese_iteration_mark_char_filter,
        ))
        .append_token_filter(BoxTokenFilter::from(japanese_compound_word_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_number_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_stop_tags_token_filter));

    // テキストをトークナイズ
    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。";
    let tokens = tokenizer.tokenize(text)?;

    // テキストとトークンを表示
    println!("text: {}", text);
    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.surface, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

上記の例は以下のように実行できます：

```shell
% cargo run --features=embed-ipadic --example=tokenize_with_filters
```

実行結果は以下のようになります：

```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: "Lindera", start: 0, end: 21, details: Some(["名詞", "固有名詞", "組織", "*", "*", "*", "*", "*", "*"])
token: "形態素", start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "ユーザー", start: 63, end: 75, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: "辞書", start: 75, end: 81, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: "利用", start: 84, end: 90, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: "可能", start: 90, end: 96, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```

## 辞書の学習（実験的機能）

Linderaは、カスタム形態素解析モデルを作成するためのCRFベースの辞書学習機能を提供しています。

### 概要

Lindera Trainerは、以下の高度な機能を備えたCondition Random Field (CRF)ベースの形態素解析器学習システムです：

- **CRFベースの統計学習**: rucrfクレートを使用した効率的な実装
- **L1正則化**: 過学習の防止
- **マルチスレッド学習**: 並行処理による学習の高速化
- **包括的なUnicodeサポート**: CJK拡張の完全サポート
- **高度な未知語処理**: インテリジェントな混合文字種分類
- **多段階の重み最適化**: 学習済み重みのための高度な正規化システム
- **Lindera辞書互換性**: 既存の辞書フォーマットとの完全互換

### CLIの使用方法

詳細なCLIコマンドの使用方法については、[lindera-cli/README.md](https://github.com/lindera/lindera/blob/main/lindera-cli/README.md#train-dictionary) を参照してください。

### 必須ファイルフォーマット仕様

#### 1. **語彙辞書 (seed.csv)**

**役割**: 基礎語彙辞書
**フォーマット**: MeCab形式のCSV

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

- **目的**: 学習のための基本的な単語とその品詞情報を定義
- **構造**: `表層形,左文脈ID,右文脈ID,コスト,品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音`

#### 2. **未知語定義 (unk.def)**

**役割**: 未知語処理の定義
**フォーマット**: 文字種ごとの未知語パラメータ

```csv
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

- **目的**: 文字種ごとの未知語の処理方法を定義
- **注意**: これらのラベルは内部処理用であり、最終的な辞書ファイルには出力されません

#### 3. **学習コーパス (corpus.txt)**

**役割**: 学習データ（注釈付きコーパス）
**フォーマット**: タブ区切りの分かち書きテキスト

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

- **目的**: 学習のための文と正解の解析結果
- **フォーマット**: 各行は `表層形\t品詞情報`、文は `EOS` で終了
- **重要**: 学習の質はこのコーパスの量と質に大きく依存します

#### 4. **文字種定義 (char.def)**

**役割**: 文字種の定義
**フォーマット**: 文字カテゴリと文字コード範囲

```text
# Character category definition (category_name compatibility_flag continuity_flag length)
DEFAULT 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# Character range mapping
0x3041..0x3096 HIRAGANA  # Hiragana
0x30A1..0x30F6 KATAKANA  # Katakana
0x4E00..0x9FAF KANJI     # Kanji
0x0030..0x0039 NUMERIC   # Numbers
0x0041..0x005A ALPHA     # Uppercase letters
0x0061..0x007A ALPHA     # Lowercase letters
```

- **目的**: どの文字がどのカテゴリに属するかを定義
- **パラメータ**: 互換性、連続性、デフォルト長などの設定

#### 5. **機能テンプレート (feature.def)**

**役割**: 素性テンプレート定義
**フォーマット**: 素性抽出パターン

```text
# Unigram features (word-level features)
UNIGRAM:%F[0]         # POS (feature element 0)
UNIGRAM:%F[1]         # POS detail 1
UNIGRAM:%F[6]         # Base form
UNIGRAM:%F[7]         # Reading (Katakana)

# Left context features
LEFT:%L[0]            # POS of left word
LEFT:%L[1]            # POS detail of left word

# Right context features
RIGHT:%R[0]           # POS of right word
RIGHT:%R[1]           # POS detail of right word

# Bigram features (combination features)
UNIGRAM:%F[0]/%F[1]   # POS + POS detail
UNIGRAM:%F[0]/%F[6]   # POS + base form
```

- **目的**: どの情報から素性を抽出するかを定義
- **テンプレート**: `%F[n]` (素性), `%L[n]` (左文脈), `%R[n]` (右文脈)

#### 6. **素性正規化ルール (rewrite.def)**

**役割**: 素性正規化ルール
**フォーマット**: 置換ルール（タブ区切り）

```text
# Normalize numeric expressions
数	NUM
*	UNK

# Normalize proper nouns
名詞,固有名詞	名詞,一般

# Simplify auxiliary verbs
助動詞,*,*,*,特殊・デス	助動詞
助動詞,*,*,*,特殊・ダ	助動詞
```

- **目的**: 素性を正規化して学習効率を向上させる
- **フォーマット**: `original_pattern\treplacement_pattern`
- **効果**: 希少な素性を一般化し、スパース性の問題を軽減する

#### 7. **出力モデルフォーマット**

**役割**: 出力モデルファイル
**フォーマット**: 標準はバイナリ(rkyv)形式、JSON形式もサポート

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

- **目的**: 後の辞書生成のために学習結果を保存

### 学習パラメータ仕様

- **正則化係数 (lambda)**: L1正則化の強さを制御 (デフォルト: 0.01)
- **最大反復回数 (iter)**: 学習の最大反復回数 (デフォルト: 100)
- **並列スレッド数 (threads)**: 並行処理スレッド数 (デフォルト: 1)

### API使用例

```rust,no_run
use std::fs::File;
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// Load configuration from files
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

// Initialize and configure trainer
let trainer = Trainer::new(config)?
    .regularization_cost(0.01)
    .max_iter(100)
    .num_threads(4);

// Load corpus
let corpus_file = File::open("resources/training/corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// Execute training
let model = trainer.train(corpus)?;

// Save model (binary format)
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Output in Lindera dictionary format
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

### 実装状況

#### 完了した機能

##### **コア機能**

- **コアアーキテクチャ**: トレーナーモジュール構造の完成
- **CRF学習**: rucrf統合によるCondition Random Field学習
- **CLI統合**: パラメータを完全にサポートした `lindera train` コマンド
- **コーパス処理**: MeCab形式コーパスの完全サポート
- **辞書統合**: seed.csv, char.def, unk.def からの辞書構築
- **素性抽出**: ユニグラム/バイグラム素性の抽出と変換
- **モデル保存**: JSON/rkyv形式での学習済みモデル出力
- **辞書出力**: Lindera形式辞書ファイルの生成

##### **高度な未知語処理**

- **包括的なUnicodeサポート**: CJK拡張、カタカナ拡張、ひらがな拡張の完全サポート
- **カテゴリ別の品詞割り当て**: 文字種による適切な品詞情報の自動割り当て
  - DEFAULT: 名詞,一般 (未知の文字種)
  - HIRAGANA/KATAKANA/KANJI: 名詞,一般 (日本語文字)
  - ALPHA: 名詞,固有名詞 (アルファベット)
  - NUMERIC: 名詞,数 (数字)
- **表層形分析**: 文字パターン、長さ、位置情報に基づく素性生成
- **動的コスト計算**: 文字種と文脈を考慮した適応的コスト計算

##### **リファクタリング（2024年9月最新）**

- **定数管理**: cost_constantsモジュールによるマジックナンバーの排除
- **メソッド分割**: 大きなメソッドの分割による可読性向上
  - `train()` → `build_lattices_from_corpus()`, `extract_labels()`, `train_crf_model()`, `create_final_model()`
- **統一されたコスト計算**: 重複コードの統一による保守性向上
  - `calculate_known_word_cost()`: 既知語コスト計算
  - `calculate_unknown_word_cost()`: 未知語コスト計算
- **整理されたデバッグ出力**: log_debug!マクロによる構造化ロギング
- **強化されたエラーハンドリング**: 包括的なエラーハンドリングとドキュメント

### アーキテクチャ

```text
lindera-dictionary/src/trainer.rs  # Main Trainer struct
lindera-dictionary/src/trainer/
├── config.rs           # Configuration management
├── corpus.rs           # Corpus processing
├── feature_extractor.rs # Feature extraction
├── feature_rewriter.rs  # Feature rewriting
└── model.rs            # Trained model
```

### 高度な未知語処理システム

#### 包括的なUnicode文字種検出

最新の実装では、基本的なUnicode範囲を大幅に拡張し、以下の文字セットを完全にサポートしています。（上記の「高度な未知語処理」セクションの「カテゴリ別の品詞割り当て」の詳細を参照してください。）

#### 素性重み最適化

##### **コスト計算定数**

```rust
mod cost_constants {
    // Known word cost calculation
    pub const KNOWN_WORD_BASE_COST: i16 = 1000;
    pub const KNOWN_WORD_COST_MULTIPLIER: f64 = 500.0;
    pub const KNOWN_WORD_COST_MIN: i16 = 500;
    pub const KNOWN_WORD_COST_MAX: i16 = 3000;
    pub const KNOWN_WORD_DEFAULT_COST: i16 = 1500;

    // Unknown word cost calculation
    pub const UNK_BASE_COST: i32 = 3000;
    pub const UNK_COST_MULTIPLIER: f64 = 500.0;
    pub const UNK_COST_MIN: i32 = 2500;
    pub const UNK_COST_MAX: i32 = 4500;

    // Category-specific adjustments
    pub const UNK_DEFAULT_ADJUSTMENT: i32 = 0;     // DEFAULT
    pub const UNK_HIRAGANA_ADJUSTMENT: i32 = 200;  // HIRAGANA - minor penalty
    pub const UNK_KATAKANA_ADJUSTMENT: i32 = 0;    // KATAKANA - medium
    pub const UNK_KANJI_ADJUSTMENT: i32 = 400;     // KANJI - high penalty
    pub const UNK_ALPHA_ADJUSTMENT: i32 = 100;     // ALPHA - mild penalty
    pub const UNK_NUMERIC_ADJUSTMENT: i32 = -100;  // NUMERIC - bonus (regular)
}
```

##### **統一されたコスト計算**

```rust
// Known word cost calculation
fn calculate_known_word_cost(&self, feature_weight: f64) -> i16 {
    let scaled_weight = (feature_weight * cost_constants::KNOWN_WORD_COST_MULTIPLIER) as i32;
    let final_cost = cost_constants::KNOWN_WORD_BASE_COST as i32 + scaled_weight;
    final_cost.clamp(
        cost_constants::KNOWN_WORD_COST_MIN as i32,
        cost_constants::KNOWN_WORD_COST_MAX as i32
    ) as i16
}

// Unknown word cost calculation
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

### パフォーマンス最適化

#### メモリ効率

- **遅延評価**: 必要な場合にのみ merged_model を作成
- **未使用素性の削除**: 学習後に不要な素性を自動削除
- **効率的なバイナリ形式**: rkyvを使用した高速シリアライゼーション

#### 並列処理サポート

```rust
let trainer = rucrf::Trainer::new()
    .regularization(rucrf::Regularization::L1, regularization_cost)?
    .max_iter(max_iter)?
    .n_threads(self.num_threads)?;  // Multi-threaded training
```

### 実践的な学習データ要件

#### 推奨コーパス仕様

実際のアプリケーション向けに効果的な辞書を生成するための推奨事項：

1. **コーパスサイズ**
   - **最小**: 100文 (基本的な動作検証用)
   - **推奨**: 1,000文以上 (実用レベル)
   - **理想**: 10,000文以上 (商用品質)

2. **語彙の多様性**
   - 異なる品詞のバランスの取れた分布
   - 活用語尾や接尾辞の網羅
   - 専門用語や固有名詞の適切な包含

3. **品質管理**
   - 形態素解析結果の手動検証
   - 解析基準の一貫した適用
   - エラー率を5%以下に維持

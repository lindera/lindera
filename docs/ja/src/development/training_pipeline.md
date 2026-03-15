# 学習パイプライン

Lindera は、カスタム形態素解析モデルを作成するための CRF ベースの辞書学習機能を提供します。この機能には `train` feature フラグが必要です。

## 概要

学習パイプラインは3つのステージで構成されます：

```text
lindera train --> model.dat --> lindera export --> dictionary files --> lindera build --> compiled dictionary
```

1. **Train**: アノテーション付きコーパスと種辞書から CRF の重みを学習し、バイナリモデルファイルを生成します。
2. **Export**: 学習済みモデルを Lindera 辞書ソースファイルに変換します。
3. **Build**: ソースファイルを Lindera が実行時に読み込めるバイナリ辞書にコンパイルします。

## 必要な入力ファイル

### 1. 種辞書 (seed.csv)

MeCab CSV 形式のベース語彙辞書です。

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

各行の構成: `surface,left_id,right_id,cost,pos,pos_detail1,pos_detail2,pos_detail3,inflection_type,inflection_form,base_form,reading,pronunciation`

種辞書では `left_id`、`right_id`、`cost` フィールドは 0 に設定されます。学習器が CRF モデルから適切な値を計算します。

### 2. 学習コーパス (corpus.txt)

タブ区切り形式のアノテーション付きテキストデータです。各行は `surface<TAB>pos_info` で、文は `EOS` で区切られます。

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

学習の品質はこのコーパスの量と質に大きく依存します。

### 3. 文字定義 (char.def)

文字タイプのカテゴリと Unicode コードポイント範囲を定義します。

```text
# Category definition: category_name compatibility_flag continuity_flag length
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

パラメータは各文字タイプの未知語がどのように分割されるかを制御します。隣接文字との互換性、同一タイプの連続が1つのトークンとして続くかどうか、デフォルトのトークン長を指定します。

### 4. 未知語定義 (unk.def)

文字タイプごとに未知語の処理方法を定義します。

```csv
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

### 5. 素性テンプレート (feature.def)

CRF モデルが学習に使用する情報を定義する MeCab 互換の素性抽出パターンです。

```text
# Unigram features (word-level)
UNIGRAM U00:%F[0]           # POS
UNIGRAM U01:%F[0],%F?[1]    # POS + POS detail (%F?[n] = optional, skipped if *)
UNIGRAM U02:%F[6]           # Base form
UNIGRAM U03:%w              # Surface form

# Bigram features (context combination)
BIGRAM B00:%L[0]/%R[0]      # Left POS / Right POS
BIGRAM B01:%L[0],%L[1]/%R[0],%R[1]  # Left POS detail / Right POS detail
```

テンプレート変数：

| 変数 | 説明 |
| --- | --- |
| `%F[n]` / `%F?[n]` | インデックス n の素性フィールド（`?` = オプション、値が `*` の場合はスキップ） |
| `%L[n]` | 左文脈の素性フィールド（rewrite.def の左セクションから） |
| `%R[n]` | 右文脈の素性フィールド（rewrite.def の右セクションから） |
| `%w` | 単語の表層形 |
| `%u` | ユニグラム書き換え後の素性文字列 |
| `%l` | 左書き換え後の素性文字列 |
| `%r` | 右書き換え後の素性文字列 |

### 6. 素性書き換えルール (rewrite.def)

MeCab 互換の3セクション形式による素性正規化ルールです。セクションは空行で区切られます。

```text
# Section 1: Unigram rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助動詞,*,*,*,特殊・デス  助動詞
*  *

# Section 2: Left context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *

# Section 3: Right context rewrite rules
名詞,固有名詞,*  名詞,固有名詞
助詞,*  助詞
*  *
```

各行は `pattern<TAB>replacement` です。パターンはワイルドカードとして `*` を使用し、前方一致で照合されます。各セクションで最初に一致したルールが適用されます。ユニグラム、左文脈、右文脈に対して異なるルールを独立して適用できるため、スパース性を低減するきめ細かい素性正規化が可能です。

## 学習パラメータ

| パラメータ | 説明 | デフォルト |
| --- | --- | --- |
| `lambda` | L1 正則化係数（過学習の制御） | 0.01 |
| `max-iterations` | 最大学習イテレーション数 | 100 |
| `max-threads` | 並列処理スレッド数 | 1 |

## CLI の使用

### Train

```bash
lindera train \
    --seed seed.csv \
    --corpus corpus.txt \
    --char-def char.def \
    --unk-def unk.def \
    --feature-def feature.def \
    --rewrite-def rewrite.def \
    --lambda 0.01 \
    --max-iter 100 \
    --max-threads 4 \
    --output model.dat
```

### Export

学習済みモデルを辞書ソースファイルに変換します：

```bash
lindera export --model model.dat --output-dir ./dict-source
```

以下のファイルが生成されます：

| ファイル | 説明 |
| --- | --- |
| `lex.csv` | 学習済みコスト付きレキシコン |
| `matrix.def` | 連接コスト行列 |
| `unk.def` | 未知語定義 |
| `char.def` | 文字定義 |
| `feature.def` | 素性テンプレート |
| `rewrite.def` | 素性書き換えルール |
| `left-id.def` | 左文脈 ID マッピング |
| `right-id.def` | 右文脈 ID マッピング |
| `metadata.json` | 辞書メタデータ |

### Build

エクスポートされたソースファイルをバイナリ辞書にコンパイルします：

```bash
lindera build --input-dir ./dict-source --output-dir ./dict-compiled
```

## 出力モデルの形式

学習済みモデルは高速読み込みのために `rkyv` バイナリ形式でシリアライズされます。以下を含みます：

- CRF で学習された素性の重み
- ラベルセット（語彙エントリー）
- 品詞情報
- 素性テンプレート
- 学習メタデータ（正則化、イテレーション数、素性・ラベル数）

## API の使用

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

## 推奨コーパス仕様

実用的なアプリケーション向けの辞書を生成するための推奨事項：

### コーパスサイズ

| レベル | 文数 | 用途 |
| --- | --- | --- |
| 最小 | 100以上 | 基本的な動作確認 |
| 推奨 | 1,000以上 | 実用的なアプリケーション |
| 理想 | 10,000以上 | 商用品質 |

### 品質ガイドライン

- **語彙の多様性**: さまざまな品詞のバランスの取れた分布、活用形・接尾語のカバー、専門用語・固有名詞の適切な含有。
- **一貫性**: コーパス全体で分析基準を一貫して適用すること。
- **検証**: 形態素解析結果を手動で検証すること。エラー率を5%以下に維持すること。

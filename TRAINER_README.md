# Lindera Trainer

Linderaの学習器機能です。コーパスから形態素解析モデルを学習できます。

## 概要

この学習器は以下の機能を提供します：

- **CRFベースの学習**: 条件付き確率場(CRF)を用いた統計的学習
- **L1正則化**: オーバーフィッティングを防ぐ正則化
- **マルチスレッド対応**: 並列処理による高速化
- **Lindera互換**: 既存のLindera辞書形式との統合

## 使用方法

### 基本的な使用方法

```bash
# 学習機能を有効にしてビルド
cargo build --features train -p lindera-cli

# 学習の実行
./target/debug/lindera train \
  --seed-lexicon examples/training/sample_lex.csv \
  --seed-unk examples/training/sample_unk.def \
  --corpus examples/training/sample_corpus.txt \
  --char-def examples/training/sample_char.def \
  --feature-def examples/training/sample_feature.def \
  --rewrite-def examples/training/sample_rewrite.def \
  --model-out trained_model.dat \
  --lambda 0.01 \
  --max-iter 100 \
  --num-threads 4

# CLIヘルプの確認
./target/debug/lindera train --help
```

### 必要なファイル（詳細）

#### 1. **`--seed-lexicon` (lex.csv)**

**役割**: 基本語彙辞書
**形式**: MeCab形式のCSV

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

- **用途**: 学習に使う基本的な単語とその品詞情報を定義
- **構成**: `表層形,左文脈ID,右文脈ID,コスト,品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音`

#### 2. **`--seed-unk` (unk.def)**

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

#### 3. **`--corpus` (corpus.txt)**

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

#### 4. **`--char-def` (char.def)**

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

#### 5. **`--feature-def` (feature.def)**

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

#### 6. **`--rewrite-def` (rewrite.def)**

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

#### 7. **`--model-out`**

**役割**: 出力モデルファイル
**形式**: JSON形式の学習済みモデル

```json
{
  "feature_weights": [0.0, 0.084, 0.091, ...],
  "labels": ["外国", "人", "参政", "権", ...],
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
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
です	助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。	記号,句点,*,*,*,*,。,。,。
EOS
```

### パラメータ

- `--lambda`: L1正則化係数（デフォルト: 0.01）
- `--max-iter`: 最大イテレーション数（デフォルト: 100）
- `--num-threads`: 使用スレッド数（デフォルト: 1）

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

### ✅ 完了済み機能

- **基本アーキテクチャ**: 完全なtrainerモジュール構造
- **CRF学習**: rucrf統合によるCondition Random Field学習
- **CLI統合**: `lindera train`コマンドで全パラメータ対応
- **コーパス処理**: MeCab形式コーパスの完全サポート
- **辞書統合**: lex.csv、char.def、unk.defからの辞書構築
- **特徴抽出**: unigram/bigram特徴の抽出と変換
- **モデル保存**: JSON形式での学習済みモデル出力
- **辞書出力**: Lindera形式辞書ファイル生成

### ✅ 新たに完了

- **辞書ローディング**: MeCab形式ファイルからの完全な辞書構築
- **モデル出力**: write_model/write_dictionaryの完全実装
- **学習パイプライン**: コーパス→特徴抽出→CRF学習→モデル保存の全工程
- **実動作確認**: サンプルデータでの学習成功確認

### ⚠️ 現在の制限事項

- **特徴テンプレート**: 基本的な実装のみ（より高度な特徴抽出が可能）
- **エラーハンドリング**: より詳細なエラー情報が必要
- **パフォーマンス**: 大規模データでの最適化余地あり

## アーキテクチャ

```text
lindera-dictionary/src/trainer/
├── mod.rs              # メインのTrainer構造体
├── config.rs           # 設定管理
├── corpus.rs           # コーパス処理
├── feature_extractor.rs # 特徴抽出
├── feature_rewriter.rs  # 特徴リライト
└── model.rs            # 学習済みモデル
```

## 動作確認結果

✅ **成功例**（2024年9月15日テスト）:

```bash
$ ./target/debug/lindera train \
  --seed-lexicon examples/training/sample_lex.csv \
  --seed-unk examples/training/sample_unk.def \
  --corpus examples/training/sample_corpus.txt \
  --char-def examples/training/sample_char.def \
  --feature-def examples/training/sample_feature.def \
  --rewrite-def examples/training/sample_rewrite.def \
  --model-out trained_model.dat

Training with 3 examples...
Building feature lattices...
Processing example 1/3
Processing example 2/3
Processing example 3/3
Starting CRF training with 3 lattices...
Training completed successfully!
Model saved to "trained_model.dat"
```

生成されたモデル（JSON形式、876バイト）:

```json
{
  "feature_weights": [0.0, 0.084, 0.091, 0.014, -0.076, ...],
  "labels": ["外国", "人", "参政", "権", "これ", ...],
  "feature_templates": ["UNIGRAM:%F[0]", "UNIGRAM:%F[1]", ...],
  "metadata": {
    "version": "1.0.0",
    "regularization": 0.01,
    "iterations": 100,
    "feature_count": 13,
    "label_count": 19
  }
}
```

## 今後の改善点

1. **特徴抽出の高度化**
   - より複雑な特徴テンプレート処理
   - 文脈を考慮した特徴抽出

2. **パフォーマンス最適化**
   - 大規模コーパスでの学習速度向上
   - メモリ使用量の最適化

3. **評価・検証機能**
   - 学習結果の精度評価
   - クロスバリデーション機能

4. **互換性の向上**
   - より多くのMeCab辞書形式への対応
   - 既存Lindera辞書との完全互換性

## ライセンス

このコードは元のLinderaプロジェクトと同じMITライセンスの下で提供されます。

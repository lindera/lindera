# Lindera Trainer

LinderaにVibratoベースの学習器を移植した機能です。コーパスから形態素解析モデルを学習できます。

## 概要

この実装では、Vibratoの学習器をLinderaに移植し、以下の機能を提供します：

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

### 必要なファイル

1. **seed_lex.csv**: シード辞書ファイル（MeCab形式）
2. **seed_unk.def**: 未知語定義ファイル
3. **corpus.txt**: 学習用コーパス（トークン化済み）
4. **char.def**: 文字定義ファイル
5. **feature.def**: 特徴定義ファイル
6. **rewrite.def**: 特徴リライトルールファイル

### コーパス形式

学習用コーパスは以下の形式で記述します：

```
外国\t名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人\t名詞,接尾,一般,*,*,*,人,ジン,ジン
参政\t名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権\t名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ\t連体詞,*,*,*,*,*,これ,コレ,コレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト\t名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
です\t助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
。\t記号,句点,*,*,*,*,。,。,。
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

```
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
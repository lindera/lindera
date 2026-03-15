# 学習

Lindera Python は、アノテーション付きコーパスからカスタム CRF ベースの形態素解析モデルを学習する機能をサポートしています。この機能には `train` feature が必要です。

## 前提条件

`train` feature を有効にして lindera-python をビルドします（デフォルトで有効）：

```bash
maturin develop --features train
```

## モデルの学習

`lindera.train()` を使用して、種辞書とアノテーション付きコーパスから CRF モデルを学習します：

```python
import lindera

lindera.train(
    seed="resources/training/seed.csv",
    corpus="resources/training/corpus.txt",
    char_def="resources/training/char.def",
    unk_def="resources/training/unk.def",
    feature_def="resources/training/feature.def",
    rewrite_def="resources/training/rewrite.def",
    output="/tmp/model.dat",
    lambda_=0.01,
    max_iter=100,
    max_threads=4,
)
```

### 学習パラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `seed` | `str` | 必須 | 種辞書ファイルのパス（CSV 形式） |
| `corpus` | `str` | 必須 | アノテーション付き学習コーパスのパス |
| `char_def` | `str` | 必須 | 文字定義ファイルのパス（char.def） |
| `unk_def` | `str` | 必須 | 未知語定義ファイルのパス（unk.def） |
| `feature_def` | `str` | 必須 | 素性定義ファイルのパス（feature.def） |
| `rewrite_def` | `str` | 必須 | 書き換えルール定義ファイルのパス（rewrite.def） |
| `output` | `str` | 必須 | 学習済みモデルファイルの出力パス |
| `lambda_` | `float` | `0.01` | L1 正則化コスト（0.0--1.0） |
| `max_iter` | `int` | `100` | 最大学習イテレーション数 |
| `max_threads` | `int` または `None` | `None` | スレッド数（None = CPU コア数を自動検出） |

## 学習済みモデルのエクスポート

学習後、`lindera.export()` を使用してモデルを辞書ソースファイルにエクスポートします：

```python
import lindera

lindera.export(
    model="/tmp/model.dat",
    output="/tmp/dictionary_source",
    metadata="resources/training/metadata.json",
)
```

### エクスポートパラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `model` | `str` | 必須 | 学習済みモデルファイルのパス（.dat） |
| `output` | `str` | 必須 | 辞書ソースファイルの出力ディレクトリ |
| `metadata` | `str` または `None` | `None` | ベースとなる metadata.json ファイルのパス |

エクスポートにより、出力ディレクトリに以下のファイルが作成されます：

- `lex.csv` -- 学習済みコスト付きのレキシコンエントリー
- `matrix.def` -- 連接コスト行列
- `unk.def` -- 未知語定義
- `char.def` -- 文字カテゴリ定義
- `metadata.json` -- 更新されたメタデータ（`metadata` パラメータ指定時）

## 完全なワークフロー

カスタム辞書の学習と使用の完全なワークフロー：

```python
import lindera

# Step 1: Train the CRF model
lindera.train(
    seed="resources/training/seed.csv",
    corpus="resources/training/corpus.txt",
    char_def="resources/training/char.def",
    unk_def="resources/training/unk.def",
    feature_def="resources/training/feature.def",
    rewrite_def="resources/training/rewrite.def",
    output="/tmp/model.dat",
    lambda_=0.01,
    max_iter=100,
)

# Step 2: Export to dictionary source files
lindera.export(
    model="/tmp/model.dat",
    output="/tmp/dictionary_source",
    metadata="resources/training/metadata.json",
)

# Step 3: Build the dictionary from exported source files
metadata = lindera.Metadata.from_json_file("/tmp/dictionary_source/metadata.json")
lindera.build_dictionary("/tmp/dictionary_source", "/tmp/dictionary", metadata)

# Step 4: Use the trained dictionary
tokenizer = (
    lindera.TokenizerBuilder()
    .set_dictionary("/tmp/dictionary")
    .set_mode("normal")
    .build()
)

tokens = tokenizer.tokenize("形態素解析のテスト")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```

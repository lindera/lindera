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
    lambda_=1.0,
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
| `lambda_` | `float` | `1.0` | 正則化係数（有限の 0 以上の値）。L2 では MeCab の `-c C` が `1 / C` にあたり、1.0 は MeCab の `-c 1.0` |
| `max_iter` | `int` | `100` | 最大学習イテレーション数（1 以上）。目的関数が収束すると、それより前に止まります（`eta` を参照） |
| `max_threads` | `int` または `None` | `None` | スレッド数（1 以上。None = CPU コア数を自動検出） |
| `regularization` | `str` | `"l2"` | 正則化の種類。`lindera train --regularization` と同じく `"l1"`・`"l2"`・`"elasticnet"` のいずれかで、大文字・小文字は区別しません（`"elastic_net"`・`"elastic-net"` も Elastic Net を選びます） |
| `elastic_net_l1_ratio` | `float` | `0.5` | Elastic Net 正則化の L1 の比率（0 以上 1 以下）。`"elasticnet"` のときだけ使います |
| `eta` | `float` | `5e-5` | 収束判定の許容値（有限の 0 以上の値）。目的関数の相対変化が 3 回続けてこの値を下回ると、`max_iter` より前に止まります。`0` で判定を無効にします |

設定の値は、ファイルを読む前に検査されます。知らない `regularization` や、範囲の外の `lambda_`・`elastic_net_l1_ratio`・`eta` は `ValueError` になります。v6 のデフォルトと同じく学習するには、`regularization="l1"` と `lambda_=0.01` を渡し、`max_iter` だけで止めるには `eta=0` も渡します。

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

エクスポートにより、出力ディレクトリに以下のファイルが作成されます。[`lindera export`](../lindera-cli/commands.md#export) が作成するファイルと同じです：

- `lex.csv` -- 学習済みコスト付きのレキシコンエントリー
- `matrix.def` -- 連接コスト行列
- `unk.def` -- 未知語定義
- `char.def` -- 文字カテゴリ定義（学習済みモデルからコピー）
- `feature.def` -- 素性テンプレート定義（学習済みモデルからコピー）
- `rewrite.def` -- 素性リライトルール（学習済みモデルからコピー）
- `left-id.def` -- 左文脈IDから素性文字列へのマッピング
- `right-id.def` -- 右文脈IDから素性文字列へのマッピング
- `metadata.json` -- 更新されたメタデータ（`metadata` パラメータ指定時）

モデルファイルは、このバージョンの学習で作ったものである必要があります。それより前のバージョンで書かれたモデルファイル（7.0.0 より前に書かれたものはすべて該当）は拒否され、`lindera.export()` は学習のやり直しを求めるエラーで失敗します。そうしたモデルは `lindera.train()` で学習し直してください。

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
    lambda_=1.0,
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
    .set_mode(lindera.Mode.normal())
    .build()
)

tokens = tokenizer.tokenize("形態素解析のテスト")
for token in tokens:
    print(f"{token.surface}\t{','.join(token.details)}")
```

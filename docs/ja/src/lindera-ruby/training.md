# 学習

Lindera Ruby は、アノテーション付きコーパスからカスタム CRF ベースの形態素解析モデルを学習する機能をサポートしています。この機能には `train` feature が必要です。

## 前提条件

`train` feature を有効にして lindera-ruby をビルドします：

```bash
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

## モデルの学習

`Lindera.train` を使用して、種辞書とアノテーション付きコーパスから CRF モデルを学習します：

```ruby
require 'lindera'

Lindera.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  0.01,   # lambda (L1 regularization)
  100,    # max_iter
  nil     # max_threads (nil = auto-detect)
)
```

### 学習パラメータ

`Lindera.train` の引数は位置引数として順番に渡します：

| 順番 | パラメータ | 型 | 説明 |
| --- | --- | --- | --- |
| 1 | seed | `String` | 種辞書ファイルのパス（CSV 形式） |
| 2 | corpus | `String` | アノテーション付き学習コーパスのパス |
| 3 | char_def | `String` | 文字定義ファイルのパス（char.def） |
| 4 | unk_def | `String` | 未知語定義ファイルのパス（unk.def） |
| 5 | feature_def | `String` | 素性定義ファイルのパス（feature.def） |
| 6 | rewrite_def | `String` | 書き換えルール定義ファイルのパス（rewrite.def） |
| 7 | output | `String` | 学習済みモデルファイルの出力パス |
| 8 | lambda | `Float` | L1 正則化コスト（0.0--1.0） |
| 9 | max_iter | `Integer` | 最大学習イテレーション数 |
| 10 | max_threads | `Integer` または `nil` | スレッド数（`nil` = CPU コア数を自動検出） |

## 学習済みモデルのエクスポート

学習後、`Lindera.export` を使用してモデルを辞書ソースファイルにエクスポートします：

```ruby
require 'lindera'

Lindera.export('/tmp/model.dat', '/tmp/dictionary_source', 'resources/training/metadata.json')
```

### エクスポートパラメータ

| 順番 | パラメータ | 型 | 説明 |
| --- | --- | --- | --- |
| 1 | model | `String` | 学習済みモデルファイルのパス（.dat） |
| 2 | output | `String` | 辞書ソースファイルの出力ディレクトリ |
| 3 | metadata | `String` または `nil` | ベースとなる metadata.json ファイルのパス |

エクスポートにより、出力ディレクトリに以下のファイルが作成されます：

- `lex.csv` -- 学習済みコスト付きのレキシコンエントリー
- `matrix.def` -- 連接コスト行列
- `unk.def` -- 未知語定義
- `char.def` -- 文字カテゴリ定義
- `metadata.json` -- 更新されたメタデータ（`metadata` パラメータ指定時）

## 完全なワークフロー

カスタム辞書の学習と使用の完全なワークフロー：

```ruby
require 'lindera'

# Step 1: Train the CRF model
Lindera.train(
  'resources/training/seed.csv',
  'resources/training/corpus.txt',
  'resources/training/char.def',
  'resources/training/unk.def',
  'resources/training/feature.def',
  'resources/training/rewrite.def',
  '/tmp/model.dat',
  0.01,  # lambda
  100,   # max_iter
  nil    # max_threads
)

# Step 2: Export to dictionary source files
Lindera.export('/tmp/model.dat', '/tmp/dictionary_source', 'resources/training/metadata.json')

# Step 3: Build the dictionary from exported source files
metadata = Lindera::Metadata.from_json_file('/tmp/dictionary_source/metadata.json')
Lindera.build_dictionary('/tmp/dictionary_source', '/tmp/dictionary', metadata)

# Step 4: Use the trained dictionary
builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/tmp/dictionary')
builder.set_mode('normal')
tokenizer = builder.build

tokens = tokenizer.tokenize('形態素解析のテスト')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```

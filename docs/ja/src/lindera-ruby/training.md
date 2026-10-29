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
  1.0,    # lambda (regularization coefficient)
  100,    # max_iter
  nil     # max_threads (nil = auto-detect)
)
```

### 学習パラメータ

最初の 7 つは必須の位置引数です。`lambda`・`max_iter`・`max_threads` はその後ろの省略できる位置引数で、正則化の種類・Elastic Net の比率・`eta` はキーワード引数です。`nil` を渡すか引数を省くと、デフォルトになります：

| 順番 | パラメータ | 型 | 説明 |
| --- | --- | --- | --- |
| 1 | seed | `String` | 種辞書ファイルのパス（CSV 形式） |
| 2 | corpus | `String` | アノテーション付き学習コーパスのパス |
| 3 | char_def | `String` | 文字定義ファイルのパス（char.def） |
| 4 | unk_def | `String` | 未知語定義ファイルのパス（unk.def） |
| 5 | feature_def | `String` | 素性定義ファイルのパス（feature.def） |
| 6 | rewrite_def | `String` | 書き換えルール定義ファイルのパス（rewrite.def） |
| 7 | output | `String` | 学習済みモデルファイルの出力パス |
| 8 | lambda | `Float` または `nil` | 正則化係数（有限の 0 以上の値）。`0` でどの種類でも正則化を無効にします。デフォルトは 1.0。L2 では MeCab の `-c C` が `1 / C` にあたり、1.0 は MeCab の `-c 1.0` |
| 9 | max_iter | `Integer` または `nil` | 最大学習イテレーション数（1 以上）。デフォルトは 100。目的関数が収束すると、それより前に止まります（`eta:` を参照） |
| 10 | max_threads | `Integer` または `nil` | スレッド数（1 以上。`nil` = CPU コア数を自動検出） |
| キーワード | regularization: | `String` または `nil` | 正則化の種類。デフォルトは `'l2'`。`lindera train --regularization` と同じく `'l1'`・`'l2'`・`'elasticnet'` のいずれかで、大文字・小文字は区別しません（`'elastic_net'`・`'elastic-net'` も Elastic Net を選びます） |
| キーワード | elastic_net_l1_ratio: | `Float` または `nil` | Elastic Net 正則化の L1 の比率（0 以上 1 以下）。`0` は L2 のみ、`1` は L1 のみを使用します。デフォルトは 0.5。`'elasticnet'` のときだけ使います |
| キーワード | eta: | `Float` または `nil` | 収束判定の許容値（有限の 0 以上の値）。デフォルトは 5e-5。目的関数の相対変化が 3 回続けてこの値を下回ると、`max_iter` より前に止まります。`0` で判定を無効にします |

設定の値は、ファイルを読む前に検査されます。知らない `regularization:` は `ArgumentError`、範囲の外の `lambda`・`elastic_net_l1_ratio:`・`eta:` は `RangeError` になります。v6 のデフォルトの設定で学習するには（最初のライン探索が変わったので、モデルは v6 と一致しません。[#1290](https://github.com/lindera/lindera/issues/1290)）、`lambda` に 0.01 を、`regularization:` に `'l1'` を渡し、`max_iter` だけで止めるには `eta: 0` も渡します：

```ruby
Lindera.train(*inputs, '/tmp/model.dat', 0.01, regularization: 'l1', eta: 0)
```

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

モデルファイルは、このバージョンの学習で作ったものである必要があります。それより前のバージョンで書かれたモデルファイル（7.0.0 より前に書かれたものはすべて該当）は拒否され、`Lindera.export` は学習のやり直しを求めるエラーで失敗します。そうしたモデルは `Lindera.train` で学習し直してください。

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
  1.0,   # lambda
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
builder.set_mode(Lindera::Mode.normal)
tokenizer = builder.build

tokens = tokenizer.tokenize('形態素解析のテスト')
tokens.each do |token|
  puts "#{token.surface}\t#{token.details.join(',')}"
end
```

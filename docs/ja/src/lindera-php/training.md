# 学習

Lindera PHP は、アノテーション付きコーパスからカスタム CRF ベースの形態素解析モデルを学習する機能をサポートしています。この機能には `train` feature が必要です。

## 前提条件

`train` feature を有効にして lindera-php をビルドします（デフォルトで有効）：

```bash
cargo build -p lindera-php --features train
```

## モデルの学習

`Lindera\Trainer::train()` を使用して、種辞書とアノテーション付きコーパスから CRF モデルを学習します：

```php
<?php

Lindera\Trainer::train(
    '/path/to/seed.csv',         // seed: 種辞書ファイル
    '/path/to/corpus.txt',       // corpus: アノテーション付きコーパス
    '/path/to/char.def',         // char_def: 文字定義ファイル
    '/path/to/unk.def',          // unk_def: 未知語定義ファイル
    '/path/to/feature.def',      // feature_def: 素性定義ファイル
    '/path/to/rewrite.def',      // rewrite_def: 書き換えルール定義ファイル
    '/tmp/model.dat',            // output: 学習済みモデルの出力パス
    1.0,                         // lambda: 正則化係数
    100,                         // max_iter: 最大イテレーション数
    null                         // max_threads: スレッド数（null = 自動検出）
);
```

### 学習パラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `$seed` | `string` | 必須 | 種辞書ファイルのパス（CSV 形式） |
| `$corpus` | `string` | 必須 | アノテーション付き学習コーパスのパス |
| `$char_def` | `string` | 必須 | 文字定義ファイルのパス（char.def） |
| `$unk_def` | `string` | 必須 | 未知語定義ファイルのパス（unk.def） |
| `$feature_def` | `string` | 必須 | 素性定義ファイルのパス（feature.def） |
| `$rewrite_def` | `string` | 必須 | 書き換えルール定義ファイルのパス（rewrite.def） |
| `$output` | `string` | 必須 | 学習済みモデルファイルの出力パス |
| `$lambda` | `float` | `1.0` | 正則化係数（有限の 0 以上の値）。`0` でどの種類でも正則化を無効にします。L2 では MeCab の `-c C` が `1 / C` にあたり、1.0 は MeCab の `-c 1.0` |
| `$max_iter` | `int` | `100` | 最大学習イテレーション数（1 以上）。目的関数が収束すると、それより前に止まります（`$eta` を参照） |
| `$max_threads` | `int` または `null` | `null` | スレッド数（1 以上。null = CPU コア数を自動検出） |
| `$regularization` | `string` または `null` | `'l2'` | 正則化の種類。`lindera train --regularization` と同じく `'l1'`・`'l2'`・`'elasticnet'` のいずれかで、大文字・小文字は区別しません（`'elastic_net'`・`'elastic-net'` も Elastic Net を選びます） |
| `$elastic_net_l1_ratio` | `float` または `null` | `0.5` | Elastic Net 正則化の L1 の比率（0 以上 1 以下）。`0` は L2 のみ、`1` は L1 のみを使用します。`'elasticnet'` のときだけ使います |
| `$eta` | `float` または `null` | `5e-5` | 収束判定の許容値（有限の 0 以上の値）。目的関数の相対変化が 3 回続けてこの値を下回ると、`$max_iter` より前に止まります。`0` で判定を無効にします |

省略できる引数は、`null` を渡すとデフォルトになります。設定の値は、ファイルを読む前に検査されます。知らない `$regularization` や、範囲の外の `$lambda`・`$elastic_net_l1_ratio`・`$eta` は `ValueError` になります。v6 のデフォルトと同じく学習するには、`regularization: 'l1'` と `lambda: 0.01` を渡し、`$max_iter` だけで止めるには `eta: 0` も渡します。

## 学習済みモデルのエクスポート

学習後、`Lindera\Trainer::export()` を使用してモデルを辞書ソースファイルにエクスポートします：

```php
<?php

Lindera\Trainer::export(
    '/tmp/model.dat',                    // model: 学習済みモデルファイル
    '/tmp/dictionary_source',            // output: 出力ディレクトリ
    '/path/to/metadata.json'             // metadata: メタデータファイル（省略可）
);
```

### エクスポートパラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `$model` | `string` | 必須 | 学習済みモデルファイルのパス（.dat） |
| `$output` | `string` | 必須 | 辞書ソースファイルの出力ディレクトリ |
| `$metadata` | `string` または `null` | `null` | ベースとなる metadata.json ファイルのパス |

エクスポートにより、出力ディレクトリに以下のファイルが作成されます。[`lindera export`](../lindera-cli/commands.md#export) が作成するファイルと同じです：

- `lex.csv` -- 学習済みコスト付きのレキシコンエントリー
- `matrix.def` -- 連接コスト行列
- `unk.def` -- 未知語定義
- `char.def` -- 文字カテゴリ定義（学習済みモデルからコピー）
- `feature.def` -- 素性テンプレート定義（学習済みモデルからコピー）
- `rewrite.def` -- 素性リライトルール（学習済みモデルからコピー）
- `left-id.def` -- 左文脈IDから素性文字列へのマッピング
- `right-id.def` -- 右文脈IDから素性文字列へのマッピング
- `metadata.json` -- 更新されたメタデータ（`$metadata` パラメータ指定時）

モデルファイルは、このバージョンの学習で作ったものである必要があります。それより前のバージョンで書かれたモデルファイル（7.0.0 より前に書かれたものはすべて該当）は拒否され、`Lindera\Trainer::export()` は学習のやり直しを求めるエラーで失敗します。そうしたモデルは `Lindera\Trainer::train()` で学習し直してください。

## 完全なワークフロー

カスタム辞書の学習と使用の完全なワークフロー：

```php
<?php

// Step 1: Train the CRF model
Lindera\Trainer::train(
    'resources/training/seed.csv',
    'resources/training/corpus.txt',
    'resources/training/char.def',
    'resources/training/unk.def',
    'resources/training/feature.def',
    'resources/training/rewrite.def',
    '/tmp/model.dat',
    1.0,    // lambda
    100,    // max_iter
    null    // max_threads
);

// Step 2: Export to dictionary source files
Lindera\Trainer::export(
    '/tmp/model.dat',
    '/tmp/dictionary_source',
    'resources/training/metadata.json'
);

// Step 3: Build the dictionary from exported source files
$metadata = Lindera\Metadata::fromJsonFile('/tmp/dictionary_source/metadata.json');
Lindera\Dictionary::build('/tmp/dictionary_source', '/tmp/dictionary', $metadata);

// Step 4: Use the trained dictionary
$dictionary = Lindera\Dictionary::load('/tmp/dictionary');
$tokenizer = new Lindera\Tokenizer($dictionary, Lindera\Mode::normal());

$tokens = $tokenizer->tokenize('形態素解析のテスト');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```

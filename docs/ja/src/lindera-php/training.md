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
    1.0,                         // lambda: L2 正則化係数
    100,                         // max_iter: 最大イテレーション数
    null                         // max_threads: スレッド数（null = 自動検出）
);
```

### 学習パラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `$seed` | `string` | 必須 | 種辞書ファイルのパス（CSV 形式） |
| `$corpus` | `string` | 必須 | アノテーション付き学習コーパスのパス |
| `$charDef` | `string` | 必須 | 文字定義ファイルのパス（char.def） |
| `$unkDef` | `string` | 必須 | 未知語定義ファイルのパス（unk.def） |
| `$featureDef` | `string` | 必須 | 素性定義ファイルのパス（feature.def） |
| `$rewriteDef` | `string` | 必須 | 書き換えルール定義ファイルのパス（rewrite.def） |
| `$output` | `string` | 必須 | 学習済みモデルファイルの出力パス |
| `$lambda` | `float` | `1.0` | L2 正則化係数（0 以上。1.0 は MeCab の `-c 1.0`）。バインディングは L2 正則化で学習し、正則化の種類はまだ選べません（[#1262](https://github.com/lindera/lindera/issues/1262)） |
| `$maxIter` | `int` | `100` | 最大学習イテレーション数。目的関数が収束すると（相対変化が 3 回続けて 5e-5 を下回ると）、それより前に止まります |
| `$maxThreads` | `int` または `null` | `null` | スレッド数（null = CPU コア数を自動検出） |

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

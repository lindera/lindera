# 辞書管理

Lindera PHP は、形態素解析で使用する辞書の読み込み、ビルド、管理のためのクラスを提供します。

## 辞書の読み込み

### システム辞書

`Lindera\Dictionary::load($uri)` を使用してシステム辞書を読み込みます。[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済み辞書をダウンロードし、展開したディレクトリのパスを指定してください：

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
```

**埋め込み辞書（上級者向け）** -- `embed-*` feature フラグ付きでビルドした場合、埋め込み辞書を使用できます：

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
```

### ユーザー辞書

ユーザー辞書はシステム辞書にカスタム語彙を追加します。`Lindera\Dictionary::loadUser()` を使用して読み込みます。

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$metadata = $dictionary->metadata();
$userDictionary = Lindera\Dictionary::loadUser('/path/to/user_dictionary.csv', $metadata);
```

トークナイザーの作成時にユーザー辞書を渡します：

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$metadata = $dictionary->metadata();
$userDictionary = Lindera\Dictionary::loadUser('/path/to/user_dictionary.csv', $metadata);

$tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDictionary);
```

または、ビルダー経由で設定します：

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$builder->setUserDictionary('/path/to/user_dictionary.csv');
$tokenizer = $builder->build();
```

### Dictionary メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `Dictionary::load($uri)` | `Dictionary` | システム辞書を読み込む |
| `Dictionary::loadUser($path, $metadata)` | `UserDictionary` | ユーザー辞書を読み込む |
| `Dictionary::version()` | `string` | Lindera のバージョン文字列を返す |
| `Dictionary::build($source, $dest, $metadata)` | `void` | 辞書をビルドする |
| `$dictionary->metadata()` | `Metadata` | 辞書のメタデータを返す |
| `$dictionary->metadataName()` | `string` | 辞書名を返す |
| `$dictionary->metadataEncoding()` | `string` | 辞書のエンコーディングを返す |

## 辞書のビルド

### システム辞書のビルド

ソースファイルからシステム辞書をビルドします：

```php
<?php

$metadata = Lindera\Metadata::fromJsonFile('/path/to/metadata.json');
Lindera\Dictionary::build('/path/to/input_dir', '/path/to/output_dir', $metadata);
```

入力ディレクトリには辞書のソースファイル（CSV レキシコン、matrix.def など）が含まれている必要があります。

`Lindera\Metadata::fromJsonFile` で読み込んだメタデータは `metadata.json` の `space_penalty` ルールを保持し、ビルドした辞書にもそのルールが同梱されます。そのため ko-dic をビルドした場合も、左側空白ペナルティはデフォルトでオンのままです。トークナイザーごとにオフにするには `TokenizerBuilder::setSpacePenalty(false)` を使います（[Tokenizer API](./tokenizer_api.md#setspacepenaltyvalue) を参照）。コンストラクタで作成した `Metadata` はルールを持ちません。

以下は IPADIC 辞書をダウンロードしてビルドする例です：

```php
<?php

$url = 'https://lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz';
$filename = '/tmp/mecab-ipadic-2.7.0-20070801.tar.gz';

// Download and extract dictionary source
file_put_contents($filename, file_get_contents($url));
$phar = new PharData($filename);
$phar->extractTo('/tmp/', null, true);

// Load metadata and build
$metadata = Lindera\Metadata::fromJsonFile('resources/ipadic_metadata.json');
Lindera\Dictionary::build(
    '/tmp/mecab-ipadic-2.7.0-20070801',
    '/tmp/lindera-ipadic',
    $metadata
);
```

## Metadata

`Metadata` クラスは辞書のパラメータを設定します。

### Metadata の作成

```php
<?php

// デフォルトのメタデータ
$metadata = Lindera\Metadata::createDefault();

// カスタムメタデータ
$metadata = new Lindera\Metadata('my_dictionary', 'UTF-8', -10000);
```

### JSON からの読み込み

```php
$metadata = Lindera\Metadata::fromJsonFile('metadata.json');
```

### プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | 辞書名 |
| `encoding` | `string` | `"UTF-8"` | 文字エンコーディング |
| `default_word_cost` | `int` | `-10000` | 未知語のデフォルトコスト |

### `toArray()`

メタデータを連想配列（値はすべて文字列）で返します。ko-dic のようにメタデータが左側空白ペナルティのルールを持つ場合は、そのルールを JSON 文字列で保持する `space_penalty` エントリも含まれます。ルールがなければこのキーはありません：

```php
<?php

$metadata = Lindera\Metadata::fromJsonFile('/path/to/ko-dic/metadata.json');
$rules = json_decode($metadata->toArray()['space_penalty'], true);
// ['rules' => [['pos' => ['EC', 'EF', ...], 'cost' => 3000], ...]]
```

## Schema

`Schema` クラスは辞書のフィールド構造を定義します。

### Schema の作成

```php
<?php

// デフォルトスキーマ（IPADIC 互換）
$schema = Lindera\Schema::createDefault();

// カスタムスキーマ
$schema = new Lindera\Schema(['surface', 'pos']);
```

### メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `fieldCount()` | `int` | フィールド数を返す |
| `getFieldIndex($name)` | `int` | フィールドのインデックスを返す（見つからない場合は `-1`） |
| `getFieldByName($name)` | `FieldDefinition` または `null` | フィールド情報を返す |
| `getCustomFields()` | `array<string>` | カスタムフィールド名の配列を返す |
| `validateRecord($record)` | `void` | レコードがスキーマに適合するか検証する |

### Schema プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `fields` | `array<string>` | フィールド名の配列 |

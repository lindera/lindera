# トークナイザー API

## TokenizerBuilder

`TokenizerBuilder` はビルダーパターンを使用して `Tokenizer` インスタンスを設定・構築します。

### コンストラクタ

#### `new Lindera\TokenizerBuilder()`

デフォルト設定で新しいビルダーを作成します。

```php
<?php

$builder = new Lindera\TokenizerBuilder();
```

### 設定メソッド

すべてのセッターメソッドはメソッドチェーンのために `$this` を返します。

#### `setMode($mode)`

トークナイズモードを設定します。

- `"normal"` -- 標準的なトークナイズ（デフォルト）
- `"decompose"` -- 複合語をより小さな単位に分解

```php
$builder->setMode('normal');
```

#### `setDictionary($uri)`

システム辞書のパスまたは URI を設定します。

```php
// 埋め込み辞書を使用
$builder->setDictionary('embedded://ipadic');

// 外部辞書を使用
$builder->setDictionary('/path/to/dictionary');
```

#### `setUserDictionary($uri)`

ユーザー辞書の URI を設定します。

```php
$builder->setUserDictionary('/path/to/user_dictionary');
```

#### `setKeepWhitespace($keep)`

出力に空白トークンを含めるかどうかを制御します。

```php
$builder->setKeepWhitespace(true);
```

#### `appendCharacterFilter($kind, $args)`

前処理パイプラインに文字フィルタを追加します。

```php
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
```

#### `appendTokenFilter($kind, $args)`

後処理パイプラインにトークンフィルタを追加します。

```php
$builder->appendTokenFilter('lowercase');
$builder->appendTokenFilter('japanese_stop_tags', ['tags' => ['助詞', '助動詞']]);
```

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```php
$tokenizer = $builder->build();
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `new Lindera\Tokenizer($dictionary, $mode, $userDictionary)`

読み込み済みの辞書から直接トークナイザーを作成します。

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');
```

ユーザー辞書を指定する場合：

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$metadata = $dictionary->metadata();
$userDictionary = Lindera\Dictionary::loadUser('/path/to/user_dictionary', $metadata);

$tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDictionary);
```

### Tokenizer メソッド

#### `tokenize($text)`

入力テキストをトークナイズし、`Token` オブジェクトの配列を返します。

```php
$tokens = $tokenizer->tokenize('形態素解析');
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `$text` | `string` | トークナイズするテキスト |

**戻り値:** `array<Token>`

#### `tokenizeNbest($text, $n)`

N-best トークナイズ結果を返します。各結果は `NbestResult` オブジェクトで、トークン配列とトータルパスコストを含みます。

```php
$results = $tokenizer->tokenizeNbest('すもももももももものうち', 3);
foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `$text` | `string` | トークナイズするテキスト |
| `$n` | `int` | 返す結果の数 |

**戻り値:** `array<NbestResult>`

## NbestResult

`NbestResult` は N-best トークナイズの個別の結果を表します。

### NbestResult プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `tokens` | `array<Token>` | トークンの配列 |
| `cost` | `int` | トータルパスコスト |

## Token

`Token` は単一の形態素トークンを表します。

### Token プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `string` | トークンの表層形 |
| `byte_start` | `int` | 元テキストでの開始バイト位置 |
| `byte_end` | `int` | 元テキストでの終了バイト位置 |
| `position` | `int` | トークンの位置インデックス |
| `word_id` | `int` | 辞書の単語 ID |
| `is_unknown` | `bool` | 辞書に登録されていない単語の場合 `true` |
| `details` | `array<string>` | 形態素の詳細情報（品詞、読みなど） |

### Token メソッド

#### `getDetail($index)`

指定されたインデックスの詳細文字列を返します。インデックスが範囲外の場合は `null` を返します。

```php
$token = $tokenizer->tokenize('東京')[0];
$pos = $token->getDetail(0);        // 例: "名詞"
$subpos = $token->getDetail(1);     // 例: "固有名詞"
$reading = $token->getDetail(7);    // 例: "トウキョウ"
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `$index` | `int` | details 配列へのゼロベースインデックス |

**戻り値:** `string` または `null`

`details` の構造は辞書によって異なります：

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: UniDic 仕様に準拠した詳細な形態素情報
- **ko-dic / CC-CEDICT / Jieba**: 各辞書固有の詳細フォーマット

## Mode

`Mode` はトークナイズの動作モードを表します。

### Mode の作成

#### `new Lindera\Mode($name)`

```php
$mode = new Lindera\Mode('normal');
$mode = new Lindera\Mode('decompose');
$mode = new Lindera\Mode();  // デフォルト: 'normal'
```

### Mode プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `name` | `string` | モード名（`"normal"` または `"decompose"`） |

### Mode メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `isNormal()` | `bool` | Normal モードの場合 `true` |
| `isDecompose()` | `bool` | Decompose モードの場合 `true` |

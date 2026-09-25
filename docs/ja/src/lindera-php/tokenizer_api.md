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
$builder->setUserDictionary('/path/to/user_dictionary.csv');
```

#### `setKeepWhitespace($keep)`

出力に空白トークンを含めるかどうかを制御します。

```php
$builder->setKeepWhitespace(true);
```

#### `setSpacePenalty($value)`

韓国語向けの左側空白ペナルティ（left-space penalty）を設定します。直前に空白がある候補のうち、本来は前の語に空白なしで付く品詞（助詞、語尾など）の候補にコストを加算します。`$value` の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.space_penalty` と同じです：

- `null` -- デフォルト。辞書が `metadata.json` に同梱するルールがあればそれを使います（ko-dic は mecab-ko-dic のルールを同梱し、他の同梱辞書はルールを持ちません）。以前の呼び出しを取り消すときにも使います。
- `false` -- ペナルティをオフにします。
- `true` -- 辞書のルールを要求します。ルールを同梱しない辞書では `build()` が `ValueError` をスローします。
- 連想配列 `['rules' => [['pos' => [...], 'cost' => n], ...]]` -- 辞書のルールの代わりにこのルールを適用します。先頭品詞タグが `pos` に含まれる候補に `cost` を加算し、最初に一致したルールが使われます。

`rules` と各 `pos` はキーが `0, 1, 2, ...` のリストでなければなりません（`array_filter()` や `array_unique()` の後は `array_values()` を適用してください）。`cost` は 32 ビットの範囲の `int` でなければならず、`6000.0` のような float は拒否されます。それ以外の値では、オブジェクト（`stdClass` や `true` を渡さない `json_decode()` の結果）や `NAN`、`INF` も含めて `ValueError` がスローされます。

```php
$builder->setDictionary('embedded://ko-dic');

// ペナルティをオフにする（v6.0 の出力）
$builder->setSpacePenalty(false);

// 明示的なルール
$builder->setSpacePenalty([
    'rules' => [
        ['pos' => ['EC', 'EF', 'EP', 'ETM', 'ETN', 'VCP', 'XSA', 'XSN', 'XSV'], 'cost' => 3000],
        ['pos' => ['JC', 'JKB', 'JKC', 'JKG', 'JKO', 'JKQ', 'JKS', 'JKV', 'JX'], 'cost' => 6000],
    ],
]);

// 辞書のデフォルトに戻す
$builder->setSpacePenalty(null);
```

> [!NOTE]
> v6.1.0 から、ko-dic では mecab-ko と同じく左側空白ペナルティがデフォルトで適用されます。たとえば `서울 시 에서` の `시` は、語尾 `EP` ではなく名詞 `NNG` と解析されるようになりました。v6.0 の出力に戻すには `$builder->setSpacePenalty(false)` を呼び出してください。[`new Lindera\Tokenizer(...)`](#new-linderatokenizerdictionary-mode-user_dictionary-space_penalty) で作成するトークナイザーでは `space_penalty: false` を渡します。ルールを同梱しているのは Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースの ko-dic ではペナルティはオフのままで、辞書を再ビルドするまで `true` はエラーになります。詳しくは [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

#### `appendCharacterFilter($kind, $args)`

前処理パイプラインに文字フィルタを追加します。

```php
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
```

#### `appendTokenFilter($kind, $args)`

後処理パイプラインにトークンフィルタを追加します。

```php
$builder->appendTokenFilter('lowercase');
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => ['助詞,格助詞,一般', '助詞,係助詞', '助詞,連体化', '助動詞'],
]);
```

`setSpacePenalty`、`appendCharacterFilter`、`appendTokenFilter` の引数は、配列の入れ子が 128 段までです。それより深い配列を渡すと `ValueError` をスローします。

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```php
$tokenizer = $builder->build();
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `new Lindera\Tokenizer($dictionary, $mode, $user_dictionary, $space_penalty)`

読み込み済みの辞書から直接トークナイザーを作成します。`$mode`、`$user_dictionary`、`$space_penalty` は省略でき、デフォルトは `null` です（`$mode` の `null` は `'normal'` を意味します）。

`$space_penalty` には [`setSpacePenalty()`](#setspacepenaltyvalue) と同じ値を渡せます。`null` では辞書が同梱する左側空白ペナルティのルール（ko-dic は同梱しています）を使います。オフにするには `false` を渡します。名前付き引数で渡せば、ほかの省略可能な引数を飛ばせます。不正な設定では、`setSpacePenalty()` や `build()` と同じく `ValueError` をスローします。

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');

// 左側空白ペナルティなしの ko-dic
$koTokenizer = new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ko-dic'), space_penalty: false);
```

ユーザー辞書を指定する場合：

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$metadata = $dictionary->metadata();
$userDictionary = Lindera\Dictionary::loadUser('/path/to/user_dictionary.csv', $metadata);

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

#### `tokenizeSurfaces($text)`

入力テキストをトークナイズし、トークンの surface のみを文字列の配列として返します。分かち書き用途の高速パスです。`Token` オブジェクトを生成せず、形態素の詳細情報もロードしないため、surface 文字列だけが必要な場合は `tokenize` より大幅に高速です。結果は `array_map(fn ($t) => $t->surface, $tokenizer->tokenize($text))` と一致します。

```php
<?php

$surfaces = $tokenizer->tokenizeSurfaces('形態素解析');
// ["形態素", "解析"]
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `$text` | `string` | トークナイズするテキスト |

**戻り値:** `array<string>`

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

#### `toArray()`

トークンを連想配列として返します。各フィールドは PHP の自然な型を保つため、
独自のエンコーダなしでシリアライズできます。

```php
$data = $tokenizer->tokenize('東京')[0]->toArray();
// ['surface' => '東京', 'byte_start' => 0, 'byte_end' => 6, 'position' => 0,
//  'word_id' => 12345, 'is_unknown' => false, 'details' => [...]]

echo json_encode(array_map(fn($t) => $t->toArray(), $tokens));
```

**戻り値:** `surface`、`byte_start`、`byte_end`、`position`、`word_id`、
`is_unknown`、`details` をキーに持つ `array`。

> [!NOTE]
> `Lindera\Token` は `JsonSerializable` を実装していないため、トークンオブジェクトを
> そのまま `json_encode` に渡してもこれらのフィールドは出力されません。先に
> `toArray()` を呼んでください。この拡張は ext-php-rs でビルドされており、その
> `#[php_class]` はまだ実装インターフェースを宣言できません（上流の
> [ext-php-rs#326](https://github.com/davidcole1340/ext-php-rs/issues/326)）。

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

## Penalty

`Penalty` は、文字種と長さのしきい値に基づいて、decompose モードが複合語をどの程度積極的に分割するかを設定します。

> **注意:** `Penalty` は現時点で `TokenizerBuilder` や `Tokenizer` のコンストラクタには接続されていません。これを受け取るセッターは存在しないため、インスタンスを作成してもトークナイズには影響しません。decompose モードは常に以下のデフォルト値を使用します。

```php
<?php

// すべてのパラメータは省略可能で、decompose モードが使用するデフォルト値になります
$penalty = new Lindera\Penalty(
    kanji_penalty_length_threshold: 2,
    kanji_penalty_length_penalty: 3000,
    other_penalty_length_threshold: 7,
    other_penalty_length_penalty: 1700,
);
```

### Penalty プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `$kanji_penalty_length_threshold` | `int` | `2` | 漢字連続のしきい値 |
| `$kanji_penalty_length_penalty` | `int` | `3000` | しきい値を超えた漢字連続に適用されるペナルティ |
| `$other_penalty_length_threshold` | `int` | `7` | その他の文字連続のしきい値 |
| `$other_penalty_length_penalty` | `int` | `1700` | しきい値を超えたその他の文字連続に適用されるペナルティ |

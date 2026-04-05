# Tokenizer API

## TokenizerBuilder

`Lindera\TokenizerBuilder` configures and constructs a `Tokenizer` instance using the builder pattern.

### Constructors

#### `new Lindera\TokenizerBuilder()`

Creates a new builder with default configuration.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
```

#### `$builder->fromFile($filePath)`

Loads configuration from a JSON file.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->fromFile('config.json');
```

### Configuration Methods

All setter methods return `$this` for method chaining.

#### `setMode($mode)`

Sets the tokenization mode.

- `"normal"` -- Standard tokenization (default)
- `"decompose"` -- Decomposes compound words into smaller units

```php
<?php

$builder->setMode('normal');
```

#### `setDictionary($path)`

Sets the system dictionary path or URI.

```php
<?php

// Use an embedded dictionary
$builder->setDictionary('embedded://ipadic');

// Use an external dictionary
$builder->setDictionary('/path/to/dictionary');
```

#### `setUserDictionary($uri)`

Sets the user dictionary URI.

```php
<?php

$builder->setUserDictionary('/path/to/user_dictionary');
```

#### `setKeepWhitespace($keep)`

Controls whether whitespace tokens appear in the output.

```php
<?php

$builder->setKeepWhitespace(true);
```

#### `appendCharacterFilter($kind, $args)`

Appends a character filter to the preprocessing pipeline.

```php
<?php

$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
```

#### `appendTokenFilter($kind, $args)`

Appends a token filter to the postprocessing pipeline.

```php
<?php

$builder->appendTokenFilter('lowercase');
```

### Build

#### `build()`

Builds and returns a `Tokenizer` with the configured settings.

```php
<?php

$tokenizer = $builder->build();
```

## Tokenizer

`Lindera\Tokenizer` performs morphological analysis on text.

### Creating a Tokenizer

#### `new Lindera\Tokenizer($dictionary, $mode, $userDictionary)`

Creates a tokenizer directly from a loaded dictionary.

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');
```

With a user dictionary:

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$metadata = $dictionary->metadata();
$userDict = Lindera\Dictionary::loadUser('/path/to/user_dictionary', $metadata);
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDict);
```

### Tokenizer Methods

#### `tokenize($text)`

Tokenizes the input text and returns an array of `Token` objects.

```php
<?php

$tokens = $tokenizer->tokenize('形態素解析');
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `$text` | `string` | Text to tokenize |

**Returns:** `array<Token>`

#### `tokenizeNbest($text, $n, $unique, $costThreshold)`

Returns the N-best tokenization results as an array of `NbestResult` objects.

```php
<?php

$results = $tokenizer->tokenizeNbest('すもももももももものうち', 3);
foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `$text` | `string` | Text to tokenize |
| `$n` | `int` | Number of results to return |
| `$unique` | `bool\|null` | Deduplicate results (default: `false`) |
| `$costThreshold` | `int\|null` | Maximum cost difference from the best path (default: `null`) |

**Returns:** `array<NbestResult>`

## NbestResult

`Lindera\NbestResult` represents a single N-best tokenization result.

### NbestResult Properties

| Property | Type | Description |
| --- | --- | --- |
| `$tokens` | `array<Token>` | The tokens in this result |
| `$cost` | `int` | The total cost of this segmentation |

## Token

`Lindera\Token` represents a single morphological token.

### Token Properties

| Property | Type | Description |
| --- | --- | --- |
| `$surface` | `string` | Surface form of the token |
| `$byte_start` | `int` | Start byte position in the original text |
| `$byte_end` | `int` | End byte position in the original text |
| `$position` | `int` | Token position index |
| `$word_id` | `int` | Dictionary word ID |
| `$is_unknown` | `bool` | `true` if the word is not in the dictionary |
| `$details` | `array<string>` | Morphological details (part of speech, reading, etc.) |

### Token Methods

#### `getDetail($index)`

Returns the detail string at the specified index, or `null` if the index is out of range.

```php
<?php

$token = $tokenizer->tokenize('東京')[0];
$pos = $token->getDetail(0);        // e.g., "名詞"
$subpos = $token->getDetail(1);     // e.g., "固有名詞"
$reading = $token->getDetail(7);    // e.g., "トウキョウ"
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `$index` | `int` | Zero-based index into the details array |

**Returns:** `string|null`

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats

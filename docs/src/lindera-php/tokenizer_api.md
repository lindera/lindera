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

$builder->setUserDictionary('/path/to/user_dictionary.csv');
```

#### `setKeepWhitespace($keep)`

Controls whether whitespace tokens appear in the output.

```php
<?php

$builder->setKeepWhitespace(true);
```

#### `setSpacePenalty($value)`

Sets the left-space penalty for Korean: a candidate that starts right after whitespace and whose part-of-speech tag normally attaches to the preceding word (particles, endings, ...) gets a cost added. `$value` has the same meaning as `segmenter.space_penalty` in a [configuration file](../lindera-analysis/configuration.md):

- `null` -- The default: the rules the dictionary ships in its `metadata.json`, if any (ko-dic ships mecab-ko-dic's rules; the other bundled dictionaries ship none). Use it to undo an earlier call.
- `false` -- Turns the penalty off.
- `true` -- Requires the dictionary's rules; `build()` throws a `ValueError` for a dictionary that ships none.
- An associative array, `['rules' => [['pos' => [...], 'cost' => n], ...]]` -- Applies these rules instead. A candidate whose first part-of-speech tag is listed in `pos` gets `cost` added; the first matching rule wins.

`rules` and each `pos` must be lists, with keys `0, 1, 2, ...` (apply `array_values()` after `array_filter()` or `array_unique()`), and `cost` must be an `int` in the 32-bit range; a float such as `6000.0` is rejected. Any other value throws a `ValueError`, including objects (a `stdClass`, or `json_decode()` without `true`) and `NAN` or `INF`.

```php
<?php

$builder->setDictionary('embedded://ko-dic');

// Turn the penalty off (the v6.0 output)
$builder->setSpacePenalty(false);

// Apply explicit rules
$builder->setSpacePenalty([
    'rules' => [
        ['pos' => ['EC', 'EF', 'EP', 'ETM', 'ETN', 'VCP', 'XSA', 'XSN', 'XSV'], 'cost' => 3000],
        ['pos' => ['JC', 'JKB', 'JKC', 'JKG', 'JKO', 'JKQ', 'JKS', 'JKV', 'JX'], 'cost' => 6000],
    ],
]);

// Back to the dictionary's default
$builder->setSpacePenalty(null);
```

> [!NOTE]
> Since v6.1.0, ko-dic applies the left-space penalty by default, as mecab-ko does. For example, `서울 시 에서` now reads `시` as the noun `NNG` rather than the ending `EP`. Call `$builder->setSpacePenalty(false)` to get the v6.0 output back; for a tokenizer created with [`new Lindera\Tokenizer(...)`](#new-linderatokenizerdictionary-mode-user_dictionary-space_penalty), pass `space_penalty: false`. Only a ko-dic built by Lindera 6.1.0 or later ships the rules; with a ko-dic from the v6.0.0 release the penalty stays off and `true` fails until the dictionary is rebuilt. See [Segmenter](../lindera/segmenter.md#left-space-penalty-korean) for details.

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

The arguments of `setSpacePenalty`, `appendCharacterFilter` and `appendTokenFilter` may nest arrays at most 128 levels deep. A deeper array throws a `ValueError`.

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

#### `new Lindera\Tokenizer($dictionary, $mode, $user_dictionary, $space_penalty)`

Creates a tokenizer directly from a loaded dictionary. `$mode`, `$user_dictionary` and `$space_penalty` are optional and default to `null`; `$mode` `null` means `'normal'`.

`$space_penalty` takes the same values as [`setSpacePenalty()`](#setspacepenaltyvalue). `null` uses the left-space penalty rules the dictionary ships (ko-dic ships them); pass `false` to turn the penalty off. Pass it by name to skip the other optional arguments. An invalid setting throws a `ValueError`, as `setSpacePenalty()` and `build()` do.

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');

// ko-dic without the left-space penalty
$koTokenizer = new Lindera\Tokenizer(Lindera\Dictionary::load('embedded://ko-dic'), space_penalty: false);
```

With a user dictionary:

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$metadata = $dictionary->metadata();
$userDict = Lindera\Dictionary::loadUser('/path/to/user_dictionary.csv', $metadata);
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

#### `tokenizeSurfaces($text)`

Tokenizes the input text and returns only the token surfaces, as an array of strings. This is the fast path for wakati-style use: no `Token` objects are created and no morphological details are loaded, so it is significantly faster than `tokenize` when only the surface strings are needed. The result equals `array_map(fn ($t) => $t->surface, $tokenizer->tokenize($text))`.

```php
<?php

$surfaces = $tokenizer->tokenizeSurfaces('形態素解析');
// ["形態素", "解析"]
```

**Parameters:**

| Name | Type | Description |
| --- | --- | --- |
| `$text` | `string` | Text to tokenize |

**Returns:** `array<string>`

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

#### `toArray()`

Returns the token as an associative array, keeping each field's natural PHP
type, so it serializes without a custom encoder.

```php
<?php

$data = $tokenizer->tokenize('東京')[0]->toArray();
// ['surface' => '東京', 'byte_start' => 0, 'byte_end' => 6, 'position' => 0,
//  'word_id' => 12345, 'is_unknown' => false, 'details' => [...]]

echo json_encode(array_map(fn($t) => $t->toArray(), $tokens));
```

**Returns:** `array` with the keys `surface`, `byte_start`, `byte_end`,
`position`, `word_id`, `is_unknown`, and `details`.

> [!NOTE]
> `Lindera\Token` does not implement `JsonSerializable`, so passing a token
> object straight to `json_encode` does not produce these fields -- call
> `toArray()` first. The extension is built with ext-php-rs, whose
> `#[php_class]` cannot declare implemented interfaces yet (upstream
> [ext-php-rs#326](https://github.com/davidcole1340/ext-php-rs/issues/326)).

The structure of `details` depends on the dictionary:

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: Detailed morphological features following the UniDic specification
- **ko-dic / CC-CEDICT / Jieba**: Dictionary-specific detail formats

## Mode

`Lindera\Mode` represents the tokenization mode.

### Creating a Mode

```php
<?php

$mode = new Lindera\Mode('normal');
$mode = new Lindera\Mode('decompose');
$mode = new Lindera\Mode();  // default: 'normal'
```

### Mode Properties

| Property | Type | Description |
| --- | --- | --- |
| `$name` | `string` | The mode name (`"normal"` or `"decompose"`) |

### Mode Methods

| Method | Return Type | Description |
| --- | --- | --- |
| `isNormal()` | `bool` | `true` if the mode is normal |
| `isDecompose()` | `bool` | `true` if the mode is decompose |

## Penalty

`Lindera\Penalty` configures how aggressively `decompose` mode splits compound words, based on character type and length thresholds.

> **Note:** `Penalty` is not currently wired into `TokenizerBuilder` or the `Tokenizer` constructor -- there is no setter that accepts it, so constructing one has no effect on tokenization yet. `decompose` mode always uses the default penalty values shown below.

```php
<?php

// All parameters are optional and default to the values used by decompose mode
$penalty = new Lindera\Penalty(
    kanji_penalty_length_threshold: 2,
    kanji_penalty_length_penalty: 3000,
    other_penalty_length_threshold: 7,
    other_penalty_length_penalty: 1700,
);
```

### Penalty Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `$kanji_penalty_length_threshold` | `int` | `2` | Length threshold for kanji sequences |
| `$kanji_penalty_length_penalty` | `int` | `3000` | Penalty applied to kanji sequences exceeding the threshold |
| `$other_penalty_length_threshold` | `int` | `7` | Length threshold for other character sequences |
| `$other_penalty_length_penalty` | `int` | `1700` | Penalty applied to other character sequences exceeding the threshold |

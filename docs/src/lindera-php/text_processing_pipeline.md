# Text Processing Pipeline

Lindera PHP supports a composable text processing pipeline that applies character filters before tokenization and token filters after tokenization. Filters are added to the `TokenizerBuilder` and executed in the order they are appended.

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

> [!NOTE]
> This page shows a few commonly used filters as examples -- it is **not** the complete list.
> `lindera-analysis` ships 4 character filters and 18 token filters in total. See
> [Filters](../lindera-analysis/filters.md) for the full, authoritative catalogue of every
> character and token filter, including parameters and examples.

## Character Filters

Character filters transform the input text before tokenization.

### unicode_normalize

Applies Unicode normalization to the input text.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$tokenizer = $builder->build();
```

Supported normalization forms: `"nfc"`, `"nfkc"`, `"nfd"`, `"nfkd"`.

### mapping

Replaces characters or strings according to a mapping table.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('mapping', [
    'mapping' => [
        'リンデラ' => 'lindera',
    ],
]);
$tokenizer = $builder->build();
```

### japanese_iteration_mark

Resolves Japanese iteration marks (odoriji) into their full forms.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendCharacterFilter('japanese_iteration_mark', [
    'normalize_kanji' => 'true',
    'normalize_kana' => 'true',
]);
$tokenizer = $builder->build();
```

## Token Filters

Token filters transform or remove tokens after tokenization.

### lowercase

Converts token surface forms to lowercase.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('lowercase');
$tokenizer = $builder->build();
```

### japanese_base_form

Replaces inflected forms with their base (dictionary) form using the morphological details from the dictionary.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_base_form', []);
$tokenizer = $builder->build();
```

### japanese_katakana_stem

Removes the trailing long sound mark from katakana words to normalize spelling variants, stemming only words at least `min` characters long.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_katakana_stem', ['min' => 3]);
$tokenizer = $builder->build();
```

### japanese_stop_tags

Removes tokens whose part-of-speech matches any of the specified tags.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => ['助詞', '助動詞'],
]);
$tokenizer = $builder->build();
```

### japanese_keep_tags

Keeps only tokens whose part-of-speech matches one of the specified tags. All other tokens are removed.

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('embedded://ipadic');
$builder->appendTokenFilter('japanese_keep_tags', [
    'tags' => ['名詞'],
]);
$tokenizer = $builder->build();
```

## Complete Pipeline Example

The following example combines multiple character filters and token filters into a single pipeline:

```php
<?php

$builder = new Lindera\TokenizerBuilder();

// Set mode and dictionary
$builder->setMode('normal');
$builder->setDictionary('embedded://ipadic');

// Preprocessing
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$builder->appendCharacterFilter(
    'japanese_iteration_mark',
    ['normalize_kanji' => 'true', 'normalize_kana' => 'true']
);
$builder->appendCharacterFilter('mapping', ['mapping' => ['リンデラ' => 'lindera']]);

// Postprocessing
$builder->appendTokenFilter('japanese_katakana_stem', ['min' => 3]);
$builder->appendTokenFilter('japanese_stop_tags', [
    'tags' => [
        '接続詞',
        '助詞',
        '助詞,格助詞',
        '助詞,格助詞,一般',
        '助詞,係助詞',
        '助詞,副助詞',
        '助詞,終助詞',
        '助詞,連体化',
        '助動詞',
        '記号',
        '記号,一般',
        '記号,読点',
        '記号,句点',
        '記号,空白',
    ],
]);
$builder->appendTokenFilter('lowercase');

// Build the tokenizer
$tokenizer = $builder->build();

// Tokenize
$text = 'Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。';
$tokens = $tokenizer->tokenize($text);

foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```

In this pipeline:

1. `unicode_normalize` converts full-width characters to half-width (NFKC normalization)
2. `japanese_iteration_mark` resolves iteration marks
3. `mapping` replaces the specified strings
4. `japanese_katakana_stem` stems katakana words
5. `japanese_stop_tags` removes particles, auxiliary verbs, and symbols
6. `lowercase` normalizes alphabetic characters to lowercase

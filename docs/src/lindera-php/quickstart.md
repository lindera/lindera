# Quick Start

This guide shows how to tokenize text using lindera-php.

## Basic Tokenization

Load a dictionary, create a tokenizer, and tokenize text:

```php
<?php

// Load the dictionary
$dictionary = Lindera\Dictionary::load('/path/to/ipadic');

// Create a tokenizer
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');

// Tokenize the text
$tokens = $tokenizer->tokenize('関西国際空港限定トートバッグ');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . implode(',', $token->details) . "\n";
}
```

> **Note:** Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory.

Expected output:

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## Using TokenizerBuilder

`TokenizerBuilder` gives you more flexible configuration options:

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setMode('normal');
$builder->setDictionary('/path/to/ipadic');
$tokenizer = $builder->build();

$tokens = $tokenizer->tokenize('すもももももももものうち');
foreach ($tokens as $token) {
    echo $token->surface . "\t" . $token->getDetail(0) . "\n";
}
```

## Accessing Token Properties

Each token exposes the following properties:

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$tokenizer = $builder->build();
$tokens = $tokenizer->tokenize('東京タワー');

foreach ($tokens as $token) {
    echo "Surface: {$token->surface}\n";
    echo "Byte range: {$token->byte_start}..{$token->byte_end}\n";
    echo "Position: {$token->position}\n";
    echo "Word ID: {$token->word_id}\n";
    echo "Unknown: " . ($token->is_unknown ? 'true' : 'false') . "\n";
    echo "Details: " . implode(',', $token->details) . "\n";
    echo "\n";
}
```

## N-best Tokenization

Retrieve multiple tokenization candidates ranked by cost:

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$tokenizer = $builder->build();
$results = $tokenizer->tokenizeNbest('すもももももももものうち', 3);

foreach ($results as $result) {
    $surfaces = array_map(fn($t) => $t->surface, $result->tokens);
    echo "Cost {$result->cost}: " . implode(' / ', $surfaces) . "\n";
}
```

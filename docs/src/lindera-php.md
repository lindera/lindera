# Lindera PHP

Lindera PHP provides PHP bindings for the Lindera morphological analysis engine, built with [ext-php-rs](https://github.com/davidcole1340/ext-php-rs). It brings Lindera's high-performance tokenization capabilities to the PHP ecosystem with support for PHP 8.1 and later.

## Features

- **Multi-language support**: Tokenize Japanese (IPADIC, IPADIC NEologd, UniDic), Korean (ko-dic), and Chinese (CC-CEDICT, Jieba) text
- **Text processing pipeline**: Compose character filters and token filters for flexible preprocessing and postprocessing
- **CRF-based dictionary training**: Train custom morphological analysis models from annotated corpora (requires `train` feature)
- **Multiple tokenization modes**: Normal and decompose modes for different analysis granularity
- **N-best tokenization**: Retrieve multiple tokenization candidates ranked by cost
- **User dictionaries**: Extend system dictionaries with custom vocabulary

## Argument types

A parameter with a declared type takes a value of that type only, with or without `declare(strict_types=1)`: an `int` parameter takes an `int`, not a numeric string or a float, and a `?Metadata` parameter takes a `Lindera\Metadata`. A `float` parameter also takes an `int`. For an optional parameter, such as `?int $cost_threshold = null`, `null` or leaving the argument out selects the default. Any other value throws an `Exception` that names the parameter. Cast values that arrive as strings, such as command-line arguments, to the declared type:

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary);

$threshold = '5000'; // for example, from $argv or a configuration file

try {
    $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: $threshold);
} catch (Exception $e) {
    echo get_class($e), ': ', $e->getMessage(), "\n";
    // Exception: Invalid value given for argument `cost_threshold`.
}

// Cast it to the declared type
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: (int) $threshold);

// null, or leaving the argument out, selects the default
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: null);
```

The parameters declared `mixed`, such as `$space_penalty` of `new Lindera\Tokenizer(...)` and `$args` of the filter methods, are not covered by this rule: they take any PHP value and check it themselves.

## Documentation

- [Installation](./lindera-php/installation.md) -- Prerequisites, build instructions, and feature flags
- [Quick Start](./lindera-php/quickstart.md) -- A minimal example to get started
- [Tokenizer API](./lindera-php/tokenizer_api.md) -- `TokenizerBuilder`, `Tokenizer`, and `Token` class reference
- [Dictionary Management](./lindera-php/dictionary_management.md) -- Loading, building, and managing dictionaries
- [Text Processing Pipeline](./lindera-php/text_processing_pipeline.md) -- Character filters and token filters
- [Training](./lindera-php/training.md) -- Training custom CRF models and exporting dictionaries

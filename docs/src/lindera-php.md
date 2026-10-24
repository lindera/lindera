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

A parameter with a declared type takes a value of that type only, with or without `declare(strict_types=1)`: an `int` parameter takes an `int`, not a numeric string or a float, and a `?Metadata` parameter takes a `Lindera\Metadata`. A `float` parameter also takes an `int`. For an optional parameter, such as `?int $cost_threshold = null`, `null` or leaving the argument out selects the default. Any other value throws a `TypeError` that names the parameter. Cast values that arrive as strings, such as command-line arguments, to the declared type:

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary);

$threshold = '5000'; // for example, from $argv or a configuration file

try {
    $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: $threshold);
} catch (TypeError $e) {
    echo get_class($e), ': ', $e->getMessage(), "\n";
    // TypeError: Invalid value given for argument `cost_threshold`.
}

// Cast it to the declared type
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: (int) $threshold);

// null, or leaving the argument out, selects the default
$results = $tokenizer->tokenizeNbest('東京都', 2, cost_threshold: null);
```

The parameters declared `mixed`, such as `$space_penalty` of `new Lindera\Tokenizer(...)` and `$args` of the filter methods, are not covered by this rule: they take any PHP value and check it themselves.

An `int` parameter also takes only the values its setting can hold, which for most settings is a narrower range than PHP's 64-bit `int`. `$default_word_cost` of `Lindera\Metadata` takes -32768 to 32767, the context IDs 0 to 65535, and the penalties of `Lindera\Penalty` a 32-bit integer. A count or an index takes 0 or more: the thresholds of `Lindera\Penalty`, `$index` of `new Lindera\FieldDefinition(...)`, `Schema::getFieldName()` and `Token::getDetail()`, `$n` of `tokenizeNbest()`, and `$max_iter` and `$max_threads` of `Lindera\Trainer::train()`. A value outside the range throws a `ValueError` that names the parameter and the range; an index past the end still returns `null`:

```php
<?php

try {
    new Lindera\Metadata(default_word_cost: 100000);
} catch (ValueError $e) {
    echo get_class($e), ': ', $e->getMessage(), "\n";
    // ValueError: default_word_cost must be an integer between -32768 and 32767, got 100000
}

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary);

try {
    $tokenizer->tokenizeNbest('東京都', -1);
} catch (ValueError $e) {
    echo get_class($e), ': ', $e->getMessage(), "\n";
    // ValueError: n must be an integer between 0 and 18446744073709551615, got -1
}
```

The float settings of `Lindera\Trainer::train()` are checked the same way, before any file is read: `$lambda` and `$eta` take a finite number of 0 or more, and `$elastic_net_l1_ratio` a number between 0 and 1 (see [Training](./lindera-php/training.md)).

## Documentation

- [Installation](./lindera-php/installation.md) -- Prerequisites, build instructions, and feature flags
- [Quick Start](./lindera-php/quickstart.md) -- A minimal example to get started
- [Tokenizer API](./lindera-php/tokenizer_api.md) -- `TokenizerBuilder`, `Tokenizer`, and `Token` class reference
- [Dictionary Management](./lindera-php/dictionary_management.md) -- Loading, building, and managing dictionaries
- [Text Processing Pipeline](./lindera-php/text_processing_pipeline.md) -- Character filters and token filters
- [Training](./lindera-php/training.md) -- Training custom CRF models and exporting dictionaries

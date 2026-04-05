# Dictionary Management

Lindera PHP provides static methods on the `Lindera\Dictionary` class for loading, building, and managing dictionaries used in morphological analysis.

## Loading Dictionaries

### System Dictionaries

Use `Lindera\Dictionary::load($uri)` to load a system dictionary. Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory:

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
```

**Embedded dictionaries (advanced)** -- if you built with an `embed-*` feature flag, you can load an embedded dictionary:

```php
<?php

$dictionary = Lindera\Dictionary::load('embedded://ipadic');
```

### User Dictionaries

User dictionaries add custom vocabulary on top of a system dictionary.

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$metadata = $dictionary->metadata();
$userDict = Lindera\Dictionary::loadUser('/path/to/user_dictionary', $metadata);
```

Pass the user dictionary when creating a tokenizer directly:

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$metadata = $dictionary->metadata();
$userDict = Lindera\Dictionary::loadUser('/path/to/user_dictionary', $metadata);

$tokenizer = new Lindera\Tokenizer($dictionary, 'normal', $userDict);
```

Or via the builder:

```php
<?php

$builder = new Lindera\TokenizerBuilder();
$tokenizer = $builder
    ->setDictionary('/path/to/ipadic')
    ->setUserDictionary('/path/to/user_dictionary')
    ->build();
```

## Building Dictionaries

### System Dictionary

Build a system dictionary from source files:

```php
<?php

$metadata = Lindera\Metadata::fromJsonFile('/path/to/metadata.json');
Lindera\Dictionary::build('/path/to/input_dir', '/path/to/output_dir', $metadata);
```

The input directory should contain the dictionary source files (CSV lexicon, matrix.def, etc.).

### User Dictionary

Build a user dictionary from a CSV file:

```php
<?php

$metadata = new Lindera\Metadata();
Lindera\Dictionary::buildUser('ipadic', 'user_words.csv', '/path/to/output_dir', $metadata);
```

## Metadata

The `Lindera\Metadata` class configures dictionary parameters.

### Creating Metadata

```php
<?php

// Default metadata
$metadata = new Lindera\Metadata();

// Custom metadata
$metadata = new Lindera\Metadata(
    name: 'my_dictionary',
    encoding: 'UTF-8',
    default_word_cost: -10000,
);

// Create with all defaults explicitly
$metadata = Lindera\Metadata::createDefault();
```

### Loading from JSON

```php
<?php

$metadata = Lindera\Metadata::fromJsonFile('metadata.json');
```

### Properties

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | Dictionary name |
| `encoding` | `string` | `"UTF-8"` | Character encoding |
| `default_word_cost` | `int` | `-10000` | Default cost for unknown words |
| `default_left_context_id` | `int` | `1288` | Default left context ID |
| `default_right_context_id` | `int` | `1288` | Default right context ID |
| `default_field_value` | `string` | `"*"` | Default value for missing fields |
| `flexible_csv` | `bool` | `false` | Allow flexible CSV parsing |
| `skip_invalid_cost_or_id` | `bool` | `false` | Skip entries with invalid cost or ID |
| `normalize_details` | `bool` | `false` | Normalize morphological details |
| `dictionary_schema_fields` | `array<string>` | IPADIC schema | Schema fields for the main dictionary |
| `user_dictionary_schema_fields` | `array<string>` | Minimal schema | Schema fields for user dictionaries |

All properties are read-only via getter methods:

```php
<?php

$metadata = new Lindera\Metadata(name: 'custom_dict', encoding: 'EUC-JP');
echo $metadata->name;      // "custom_dict"
echo $metadata->encoding;  // "EUC-JP"
```

### `toArray()`

Returns an associative array representation of the metadata:

```php
<?php

$metadata = new Lindera\Metadata(name: 'test');
print_r($metadata->toArray());
```

### Dictionary Info

The `Lindera\Dictionary` object provides metadata accessors:

```php
<?php

$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
echo $dictionary->metadataName();      // Dictionary name
echo $dictionary->metadataEncoding();  // Dictionary encoding
$metadata = $dictionary->metadata();   // Full Metadata object
```

### Version

Retrieve the Lindera library version:

```php
<?php

echo Lindera\Dictionary::version();
```

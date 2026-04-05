# Lindera PHP

PHP bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.

## Requirements

- PHP 8.1+
- Rust toolchain (stable)

## Build

```bash
cargo build -p lindera-php --features embed-ipadic
```

The shared library will be at `target/debug/liblindera_php.so` (or `.dylib` on macOS).

## Installation

Copy the shared library to your PHP extensions directory:

```bash
cp target/release/liblindera_php.so $(php -r 'echo ini_get("extension_dir");')/lindera_php.so
```

Enable in `php.ini`:

```ini
extension=lindera_php.so
```

Or load at runtime:

```bash
php -d extension=target/debug/liblindera_php.so your_script.php
```

## Usage

### Basic Tokenization

```php
$tokenizer = (new Lindera\TokenizerBuilder())->build();
$tokens = $tokenizer->tokenize("関西国際空港");

foreach ($tokens as $token) {
    echo $token->surface . " [" . implode(",", $token->details) . "]\n";
}
```

### With Dictionary

```php
$dict = Lindera\load_dictionary("ipadic");
$tokenizer = new Lindera\Tokenizer($dict, "normal");
$tokens = $tokenizer->tokenize("すもももももももものうち");
```

### Decompose Mode

```php
$builder = new Lindera\TokenizerBuilder();
$builder->set_mode("decompose");
$tokenizer = $builder->build();
```

### With Filters

```php
$builder = new Lindera\TokenizerBuilder();
$builder->set_mode("normal");
$builder->append_character_filter("unicode_normalize", ["kind" => "nfkc"]);
$builder->append_token_filter("japanese_stop_tags", ["tags" => ["助詞"]]);
$tokenizer = $builder->build();
```

### N-Best Tokenization

```php
$tokenizer = (new Lindera\TokenizerBuilder())->build();
$results = $tokenizer->tokenize_nbest("東京都", 3);

foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

## API Reference

### Classes

| Class | Description |
|-------|-------------|
| `Lindera\TokenizerBuilder` | Builder for creating tokenizers |
| `Lindera\Tokenizer` | Morphological analyzer |
| `Lindera\Token` | Analysis result token |
| `Lindera\NbestResult` | N-best tokenization result |
| `Lindera\Dictionary` | Morphological dictionary |
| `Lindera\UserDictionary` | User-defined dictionary |
| `Lindera\Mode` | Tokenization mode |
| `Lindera\Penalty` | Decompose mode penalty |
| `Lindera\Metadata` | Dictionary metadata |
| `Lindera\Schema` | Dictionary schema |
| `Lindera\FieldDefinition` | Schema field definition |
| `Lindera\FieldType` | Schema field type |
| `Lindera\CompressionAlgorithm` | Compression algorithm |

### Functions

| Function | Description |
|----------|-------------|
| `Lindera\version()` | Returns the package version |
| `Lindera\load_dictionary(uri)` | Loads a dictionary |
| `Lindera\load_user_dictionary(uri, metadata)` | Loads a user dictionary |
| `Lindera\build_dictionary(input, output, metadata)` | Builds a dictionary |
| `Lindera\build_user_dictionary(kind, input, output, metadata?)` | Builds a user dictionary |
| `Lindera\train(...)` | Trains a model (train feature) |
| `Lindera\export(model, output, metadata?)` | Exports dictionary files |

## License

MIT

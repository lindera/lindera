# Lindera PHP

PHP extension for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.

## Requirements

- PHP 8.1+ on Linux or macOS (Windows is not supported)
- To build from source: Rust toolchain (stable) and libclang

## Installation

### With PIE (recommended)

The extension is published on Packagist as `lindera/lindera` (from lindera v6.1.0) and installed with [PIE](https://github.com/php/pie), the PHP extension installer. Composer itself does not install `php-ext` packages, so `composer require lindera/lindera` is not the way in.

```bash
pie install lindera/lindera
```

PIE builds the extension from source for your PHP, which needs a Rust toolchain (<https://rustup.rs/>), libclang (`libclang-dev` on Debian/Ubuntu; the Xcode command line tools or `brew install llvm` on macOS) and PIE's usual build tools (`autoconf`, `libtool`, `make`). It then installs `lindera.so` into the extension directory and enables it. Pass `--with-php-config=/path/to/php-config` to target another PHP.

### Manual build

```bash
git clone https://github.com/lindera/lindera.git
cd lindera
cargo build --release -p lindera-php
```

The shared library will be at `target/release/liblindera_php.so` (or `.dylib` on macOS). Either add it to `php.ini`:

```ini
extension=/path/to/lindera/target/release/liblindera_php.so
```

or pass it per invocation:

```bash
php -d extension=/path/to/liblindera_php.so your_script.php
```

Either way the extension registers as `lindera`: `php -m` lists `lindera`, and `extension_loaded('lindera')` is the check to use.

### Dictionaries

The extension does not embed a dictionary. Download a pre-built one (for example `lindera-ipadic-<version>.zip`) from [GitHub Releases](https://github.com/lindera/lindera/releases), extract it, and pass its path to `Lindera\Dictionary::load()` or `TokenizerBuilder::setDictionary()`. A self-built extension can embed dictionaries with the `embed-*` features instead (for example `--features embed-ipadic`), which are then loaded as `embedded://ipadic`.

## Usage

### Basic Tokenization

```php
$dictionary = Lindera\Dictionary::load('/path/to/ipadic');
$tokenizer = new Lindera\Tokenizer($dictionary, 'normal');
$tokens = $tokenizer->tokenize('関西国際空港');

foreach ($tokens as $token) {
    echo $token->surface . ' [' . implode(',', $token->details) . "]\n";
}
```

### Converting Tokens to Plain Data

`toArray()` returns the token as an associative array, keeping each field's
natural PHP type, so it serializes without a custom encoder:

```php
$data = $tokens[0]->toArray();
// ['surface' => ..., 'byte_start' => 0, 'byte_end' => 9, 'position' => 0,
//  'word_id' => 12345, 'is_unknown' => false, 'details' => [...]]

echo json_encode(array_map(fn($t) => $t->toArray(), $tokens));
```

`Lindera\Token` does not implement `JsonSerializable`, so passing a token
object straight to `json_encode` does not produce these fields — call
`toArray()` first, as above. The extension is built with ext-php-rs, whose
`#[php_class]` cannot declare implemented interfaces yet (upstream
[ext-php-rs#326](https://github.com/davidcole1340/ext-php-rs/issues/326)).

### TokenizerBuilder

```php
$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$builder->setMode('decompose');
$tokenizer = $builder->build();
```

### With Filters

```php
$builder = new Lindera\TokenizerBuilder();
$builder->setDictionary('/path/to/ipadic');
$builder->appendCharacterFilter('unicode_normalize', ['kind' => 'nfkc']);
$builder->appendTokenFilter('japanese_stop_tags', ['tags' => ['助詞,格助詞,一般', '助詞,係助詞', '助詞,連体化', '助動詞']]);
$tokenizer = $builder->build();
```

### N-Best Tokenization

```php
$tokenizer = new Lindera\Tokenizer(Lindera\Dictionary::load('/path/to/ipadic'));
$results = $tokenizer->tokenizeNbest('東京都', 3);

foreach ($results as $result) {
    echo "Cost: {$result->cost}\n";
    foreach ($result->tokens as $token) {
        echo "  {$token->surface}\n";
    }
}
```

## API Reference

All classes live in the `Lindera` namespace; the extension registers no functions.

| Class | Description |
| --- | --- |
| `Lindera\TokenizerBuilder` | Builder for creating tokenizers |
| `Lindera\Tokenizer` | Morphological analyzer |
| `Lindera\Token` | Analysis result token |
| `Lindera\NbestResult` | N-best tokenization result |
| `Lindera\Dictionary` | Morphological dictionary: `load()`, `loadUser()`, `build()`, `buildUser()`, `version()` |
| `Lindera\UserDictionary` | User-defined dictionary |
| `Lindera\Mode` | Tokenization mode |
| `Lindera\Penalty` | Decompose mode penalty |
| `Lindera\Metadata` | Dictionary metadata |
| `Lindera\Schema` | Dictionary schema |
| `Lindera\FieldDefinition` | Schema field definition |
| `Lindera\FieldType` | Schema field type |
| `Lindera\Trainer` | Model training: `train()`, `export()` (`train` feature, enabled by default) |

### Stubs for static analysis

[`stubs/lindera.stubs.php`](stubs/lindera.stubs.php) declares every class of the extension for IDEs, PHPStan and Psalm. It is generated from the compiled extension and the test suite fails when it is out of date, so it cannot drift from the Rust source. Point your analyser at a copy of the file, and never include it at runtime — the extension already declares the classes.

```neon
# phpstan.neon
parameters:
    stubFiles:
        - path/to/lindera/lindera-php/stubs/lindera.stubs.php
```

## Development

```bash
make test-lindera-php      # cargo test + build the extension + PHPUnit
make test-lindera-php-pie  # the phpize / configure / make path that PIE runs
make stubs-lindera-php     # regenerate stubs/lindera.stubs.php after changing the API
make build-lindera-php     # release build
```

`composer.json` lives at the repository root, not in this directory: Packagist reads it from there, and its `php-ext.build-path` points PIE back at `lindera-php/`, where `config.m4` and `Makefile.frag` run `cargo build`.

## License

MIT

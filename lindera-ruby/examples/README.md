# Lindera Ruby Examples

## Prerequisites

- Ruby >= 3.1
- Rust toolchain
- Bundler (`gem install bundler`)

## Build

Run from the `lindera-ruby/` directory:

```bash
cd lindera-ruby
bundle install

# Build with embedded IPADIC dictionary
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile

# Build with training feature
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile
```

The compiled extension is placed in `lib/lindera/` by Rake.

## Examples

Run from the `lindera-ruby/` directory after building:

### Basic tokenization

```bash
bundle exec ruby examples/tokenize.rb
```

Tokenizes Japanese text using the embedded IPADIC dictionary in "normal" mode.

### Decompose mode

```bash
bundle exec ruby examples/tokenize_with_decompose.rb
```

Tokenizes text in "decompose" mode, which decomposes compound morphemes.

### Tokenization with filters

```bash
bundle exec ruby examples/tokenize_with_filters.rb
```

Demonstrates `TokenizerBuilder` API with character filters (`unicode_normalize`, `japanese_iteration_mark`, `mapping`) and token filters (`japanese_katakana_stem`, `japanese_stop_tags`, `lowercase`, `japanese_base_form`).

### User dictionary

```bash
bundle exec ruby examples/tokenize_with_userdict.rb
```

Loads a custom user dictionary from `resources/ipadic_simple_userdic.csv` alongside the standard dictionary.

### Build dictionary from source

```bash
bundle exec ruby examples/build_ipadic.rb
```

Downloads the mecab-ipadic source tarball and builds a Lindera dictionary from it. Requires `tar` command.

### Train and export (requires `train` feature)

```bash
# Build with train feature first
LINDERA_FEATURES="embed-ipadic,train" bundle exec rake compile

bundle exec ruby examples/train_and_export.rb
```

Trains a CRF model from a sample corpus and exports dictionary files.

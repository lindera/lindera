# lindera-ruby

Ruby binding for [Lindera](https://github.com/lindera/lindera), a morphological analysis engine for CJK text.

## Overview

lindera-ruby provides a Ruby interface to the Lindera morphological analysis engine, supporting Japanese, Korean, and Chinese text analysis.

- **Multi-language Support**: Japanese (IPADIC, UniDic), Korean (ko-dic), Chinese (CC-CEDICT, Jieba)
- **Character Filters**: Text preprocessing with mapping, regex, Unicode normalization, and Japanese iteration mark handling
- **Token Filters**: Post-processing filters including lowercase, length filtering, stop words, and Japanese-specific filters
- **Flexible Configuration**: Configurable tokenization modes and penalty settings
- **Metadata Support**: Complete dictionary schema and metadata management
- **Training & Export** (optional): Train custom morphological analysis models from corpus data

## Requirements

- Ruby >= 3.1
- Rust >= 1.85

## Install

```bash
cd lindera-ruby
bundle install
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
```

## Usage

```ruby
require "lindera"

# Load dictionary
dictionary = Lindera.load_dictionary("embedded://ipadic")

# Create a tokenizer
tokenizer = Lindera::Tokenizer.new(dictionary, "normal", nil)

# Tokenize text
tokens = tokenizer.tokenize("関西国際空港")
tokens.each do |token|
  puts "#{token.surface}: #{token.details&.join(', ')}"
end
```

### Using TokenizerBuilder

```ruby
require "lindera"

builder = Lindera::TokenizerBuilder.new
builder.set_mode("normal")
builder.set_dictionary("embedded://ipadic")

# Add filters
builder.append_character_filter("unicode_normalize", { "kind" => "nfkc" })
builder.append_token_filter("lowercase", nil)

tokenizer = builder.build
tokens = tokenizer.tokenize("テスト")
```

## Test

```bash
LINDERA_FEATURES="embed-ipadic" bundle exec rake compile
bundle exec rake test
```

## License

MIT

# lindera-ruby

Ruby binding for [Lindera](https://github.com/lindera/lindera), a morphological analysis engine for CJK text.

## Overview

lindera-ruby provides a Ruby interface to the Lindera morphological analysis engine, supporting Japanese, Korean, and Chinese text analysis.

- **Multi-language Support**: Japanese (IPADIC, IPADIC-NEologd, UniDic), Korean (ko-dic), Chinese (CC-CEDICT, Jieba)
- **Character Filters**: Text preprocessing with mapping, regex, Unicode normalization, and Japanese iteration mark handling
- **Token Filters**: Post-processing filters including lowercase, length filtering, stop words, and Japanese-specific filters
- **Flexible Configuration**: Configurable tokenization modes and penalty settings
- **Metadata Support**: Complete dictionary schema and metadata management
- **Training & Export** (optional): Train custom morphological analysis models from corpus data

## Requirements

- Ruby >= 3.1
- Rust >= 1.85

## Dictionary

Pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`) and extract it to a local path.

## Install

```bash
cd lindera-ruby
bundle install
bundle exec rake compile
```

## Usage

```ruby
require "lindera"

# Load dictionary from a local path (download from GitHub Releases)
dictionary = Lindera.load_dictionary("/path/to/ipadic")

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
builder.set_dictionary("/path/to/ipadic")

# Add filters
builder.append_character_filter("unicode_normalize", { "kind" => "nfkc" })
builder.append_token_filter("lowercase", nil)

tokenizer = builder.build
tokens = tokenizer.tokenize("テスト")
```

## Test

```bash
bundle exec rake compile
bundle exec rake test
```

## License

MIT

# Lindera Ruby

Lindera Ruby provides Ruby bindings for the Lindera morphological analysis engine, built with [Magnus](https://github.com/matsadler/magnus) and [rb-sys](https://github.com/oxidize-rb/rb-sys). It brings Lindera's high-performance tokenization capabilities to the Ruby ecosystem with support for Ruby 3.1 and later.

## Features

- **Multi-language support**: Tokenize Japanese (IPADIC, IPADIC NEologd, UniDic), Korean (ko-dic), and Chinese (CC-CEDICT, Jieba) text
- **Text processing pipeline**: Compose character filters and token filters for flexible preprocessing and postprocessing
- **CRF-based dictionary training**: Train custom morphological analysis models from annotated corpora (requires `train` feature)
- **Multiple tokenization modes**: Normal and decompose modes for different analysis granularity
- **N-best tokenization**: Retrieve multiple tokenization candidates ranked by cost
- **User dictionaries**: Extend system dictionaries with custom vocabulary

## Integer arguments

An integer parameter takes only the values its setting can hold. `default_word_cost` of `Lindera::Metadata.new` takes -32768 to 32767, the context IDs 0 to 65535, and the penalties of `Lindera::Penalty.new` a 32-bit integer. A count or an index takes 0 or more: the thresholds of `Lindera::Penalty.new`, `index` of `Schema#get_field_name` and `Token#get_detail`, `n` of `Tokenizer#tokenize_nbest`, and `max_iter` and `max_threads` of `Lindera.train`. `cost_threshold` of `Tokenizer#tokenize_nbest` takes a 64-bit integer. A value outside the range raises a `RangeError` that names the parameter and the range; an index past the end still returns `nil`:

```ruby
require 'lindera'

begin
  Lindera::Metadata.new(nil, nil, 100_000, nil, nil, nil, nil, nil)
rescue RangeError => e
  puts "#{e.class}: #{e.message}"
  # RangeError: default_word_cost must be an integer between -32768 and 32767, got 100000
end

tokenizer = Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ipadic'))

begin
  tokenizer.tokenize_nbest('東京都', -1, nil, nil)
rescue RangeError => e
  puts "#{e.class}: #{e.message}"
  # RangeError: n must be an integer between 0 and 18446744073709551615, got -1
end
```

The float settings of `Lindera.train` are checked the same way, before any file is read: `lambda` and `eta:` take a finite number of 0 or more, and `elastic_net_l1_ratio:` a number between 0 and 1 (see [Training](./lindera-ruby/training.md)).

## Documentation

- [Installation](./lindera-ruby/installation.md) -- Prerequisites, build instructions, and feature flags
- [Quick Start](./lindera-ruby/quickstart.md) -- A minimal example to get started
- [Tokenizer API](./lindera-ruby/tokenizer_api.md) -- `TokenizerBuilder`, `Tokenizer`, and `Token` class reference
- [Dictionary Management](./lindera-ruby/dictionary_management.md) -- Loading, building, and managing dictionaries
- [Text Processing Pipeline](./lindera-ruby/text_processing_pipeline.md) -- Character filters and token filters
- [Training](./lindera-ruby/training.md) -- Training custom CRF models and exporting dictionaries

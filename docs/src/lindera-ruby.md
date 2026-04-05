# Lindera Ruby

Lindera Ruby provides Ruby bindings for the Lindera morphological analysis engine, built with [Magnus](https://github.com/matsadler/magnus) and [rb-sys](https://github.com/oxidize-rb/rb-sys). It brings Lindera's high-performance tokenization capabilities to the Ruby ecosystem with support for Ruby 3.1 and later.

## Features

- **Multi-language support**: Tokenize Japanese (IPADIC, IPADIC NEologd, UniDic), Korean (ko-dic), and Chinese (CC-CEDICT, Jieba) text
- **Text processing pipeline**: Compose character filters and token filters for flexible preprocessing and postprocessing
- **CRF-based dictionary training**: Train custom morphological analysis models from annotated corpora (requires `train` feature)
- **Multiple tokenization modes**: Normal and decompose modes for different analysis granularity
- **N-best tokenization**: Retrieve multiple tokenization candidates ranked by cost
- **User dictionaries**: Extend system dictionaries with custom vocabulary

## Documentation

- [Installation](./lindera-ruby/installation.md) -- Prerequisites, build instructions, and feature flags
- [Quick Start](./lindera-ruby/quickstart.md) -- A minimal example to get started
- [Tokenizer API](./lindera-ruby/tokenizer_api.md) -- `TokenizerBuilder`, `Tokenizer`, and `Token` class reference
- [Dictionary Management](./lindera-ruby/dictionary_management.md) -- Loading, building, and managing dictionaries
- [Text Processing Pipeline](./lindera-ruby/text_processing_pipeline.md) -- Character filters and token filters
- [Training](./lindera-ruby/training.md) -- Training custom CRF models and exporting dictionaries

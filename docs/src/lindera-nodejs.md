# Lindera Node.js

Lindera Node.js provides Node.js bindings for the Lindera morphological analysis engine, built with [NAPI-RS](https://napi.rs/). It brings Lindera's high-performance tokenization capabilities to the Node.js ecosystem with support for Node.js 18 and later.

## Features

- **Multi-language support**: Tokenize Japanese (IPADIC, IPADIC NEologd, UniDic), Korean (ko-dic), and Chinese (CC-CEDICT, Jieba) text
- **Text processing pipeline**: Compose character filters and token filters for flexible preprocessing and postprocessing
- **CRF-based dictionary training**: Train custom morphological analysis models from annotated corpora (requires `train` feature)
- **Multiple tokenization modes**: Normal and decompose modes for different analysis granularity
- **N-best tokenization**: Retrieve multiple tokenization candidates ranked by cost
- **User dictionaries**: Extend system dictionaries with custom vocabulary
- **TypeScript support**: Full type definitions included out of the box

## Documentation

- [Installation](./lindera-nodejs/installation.md) -- Prerequisites, build instructions, and feature flags
- [Quick Start](./lindera-nodejs/quickstart.md) -- A minimal example to get started
- [Tokenizer API](./lindera-nodejs/tokenizer_api.md) -- `TokenizerBuilder`, `Tokenizer`, and `Token` class reference
- [Dictionary Management](./lindera-nodejs/dictionary_management.md) -- Loading, building, and managing dictionaries
- [Text Processing Pipeline](./lindera-nodejs/text_processing_pipeline.md) -- Character filters and token filters
- [Training](./lindera-nodejs/training.md) -- Training custom CRF models and exporting dictionaries

# Lindera Node.js

Lindera Node.js provides Node.js bindings for the Lindera morphological analysis engine, built with [NAPI-RS](https://napi.rs/). It brings Lindera's high-performance tokenization capabilities to the Node.js ecosystem, on the Node.js versions `^20.17.0 || ^22.13.0 || >= 23.5.0`.

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

## Errors

A function or method that fails throws an `Error`. Its `message` says what went wrong, and its `code` tells the kind of failure:

| `code` | Failure | Example |
| --- | --- | --- |
| The type expected, such as `StringExpected` or `ArrayExpected` | An argument of another type | `loadDictionary(123)`, `new Schema("a")` |
| `InvalidArg` | An argument that is not an instance of the class expected, or a number outside the range of its setting | `setMode("normal")`, `new Metadata({ defaultLeftContextId: 2 ** 32 + 5 })` |
| `GenericFailure` | Most other failures, and a panic (below) | `loadDictionary("/nonexistent/dictionary")`, `Mode.fromStr("Normal")` |

A panic in Lindera's Rust code, which is a bug, throws an `Error` too, from the function or method that was called, and Node.js goes on running. Up to v6.2.0, a panic aborted Node.js with `fatal runtime error: failed to initiate panic, error 5, aborting` ([#1314](https://github.com/lindera/lindera/issues/1314)). The `Error` of a panic has the `code` `GenericFailure`, as most other failures do, and the panic's message as its `message`. Rust also writes the place of the panic in its source code and the message to standard error.

An object whose method panicked may keep what the method changed before the panic, so create it again instead of using it further. Please report a panic at [GitHub Issues](https://github.com/lindera/lindera/issues), with its message and the input that caused it.

# Lindera Library

The `lindera` crate is a pure morphological segmenter: it integrates the dictionary crates and provides the `Segmenter` API. It does not depend on `lindera-analysis`, `lindera-crf`, or `lindera-trainer` by default. This section covers segmentation, error handling, and API reference.

If you need the `Tokenizer`, character filters, or token filters (a Lucene-style analysis chain built on top of `Segmenter`), see the separate [Lindera Analysis](./lindera-analysis.md) crate, including its [Configuration](./lindera-analysis/configuration.md) and [Filters](./lindera-analysis/filters.md) pages.

- [Segmenter](./lindera/segmenter.md) - Core segmentation component using the Viterbi algorithm
- [Error Handling](./lindera/error_handling.md) - Error types and handling patterns
- [API Reference](./lindera/api_reference.md) - Links to generated API documentation

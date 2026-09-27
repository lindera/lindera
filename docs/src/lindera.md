# Lindera Library

The `lindera` crate is the facade of the Rust API: it re-exports the morphological segmenter from `lindera-segmenter` (`lindera::segmenter`, `lindera::dictionary`, `lindera::mode`, ...) and, with the default `analysis` feature, the analysis chain from `lindera-analysis` as `lindera::analysis`. `lindera = "7"` is the only dependency you need; `default-features = false` gives you the segmenter alone. This section covers segmentation, error handling, and API reference.

If you need the `Tokenizer`, character filters, or token filters (a Lucene-style analysis chain built on top of `Segmenter`), use `lindera::analysis` — see the [Lindera Analysis](./lindera-analysis.md) chapter, including its [Configuration](./lindera-analysis/configuration.md) and [Filters](./lindera-analysis/filters.md) pages.

- [Segmenter](./lindera/segmenter.md) - Core segmentation component using the Viterbi algorithm
- [Error Handling](./lindera/error_handling.md) - Error types and handling patterns
- [API Reference](./lindera/api_reference.md) - Links to generated API documentation

# Lindera Analysis

Lindera Analysis layers a Lucene-style text analysis chain on top of the pure morphological segmenter provided by the [`lindera`](./lindera.md) crate. It composes character filters, a `Segmenter`, and token filters into a single `Tokenizer` pipeline that can be built programmatically or loaded entirely from a YAML configuration file.

## Key Features

- **Character filters** that transform input text before segmentation, with automatic byte-offset correction back to the original text
- **Token filters** that transform, merge, filter, or reorder the tokens produced by the segmenter
- **`Tokenizer` / `TokenizerBuilder`** to assemble a full analysis pipeline, either in Rust code or from a YAML file (`LINDERA_CONFIG_PATH`)
- Built-in filters covering Japanese, Korean, and general-purpose text normalization

## Contents

- [Configuration](./lindera-analysis/configuration.md) -- YAML configuration file format for the `Tokenizer`
- [Filters](./lindera-analysis/filters.md) -- Reference for all character filters and token filters
- [Architecture](./lindera-analysis/architecture.md) -- Internal structure and key components
- [API Reference](./lindera-analysis/api_reference.md) -- API documentation

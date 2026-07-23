# Examples

Lindera includes several example programs that demonstrate common use cases. The source code is available in the [examples directory](https://github.com/lindera/lindera/tree/main/lindera/examples) on GitHub.

The `tokenize*` examples use the `Tokenizer` and filter APIs, which are
provided by the `lindera-analysis` crate (as of v5.0).

All examples below are run with the `embed-ipadic` feature enabled, which downloads the IPADIC dictionary and embeds it into the binary automatically at build time — no manual dictionary download is required.

## Available Examples

### segment

Basic morphological segmentation with the `Segmenter` API — the `lindera`
crate alone is enough.

```shell
cargo run -p lindera --features=embed-ipadic --example=segment
```

### tokenize

Basic tokenization using an external IPADIC dictionary. Segments input text and prints each token with its part-of-speech details.

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize
```

### tokenize_with_user_dict

Tokenization with a user dictionary. Shows how to supplement the dictionary with custom entries for domain-specific terms.

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_user_dict
```

### tokenize_with_filters

Tokenization with character filters and token filters. Demonstrates the text processing pipeline, including Unicode normalization, part-of-speech filtering, and other transformations.

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_filters
```

### tokenize_with_config

Tokenization using a YAML configuration file. Shows how to configure the tokenizer declaratively instead of programmatically.

```shell
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_config
```

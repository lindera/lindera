# Examples

Lindera includes several example programs that demonstrate common use cases. The source code is available in the [examples directory](https://github.com/lindera/lindera/tree/main/lindera/examples) on GitHub (the `tokenize*` examples are in [lindera-analysis/examples](https://github.com/lindera/lindera/tree/main/lindera-analysis/examples)).

The `tokenize*` examples use the `Tokenizer` and filter APIs from
`lindera::analysis`. They live in the `lindera-analysis` crate, hence
`-p lindera-analysis` below.

All examples below are run with the `embed-ipadic` feature enabled, which downloads the IPADIC dictionary and embeds it into the binary automatically at build time — no manual dictionary download is required.

## Available Examples

### segment

Basic morphological segmentation with the `Segmenter` API — it needs only the
segmenter, so it also works with `default-features = false`.

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

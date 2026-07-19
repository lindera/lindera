# Examples

Lindera includes several example programs that demonstrate common use cases. The source code is available in the [examples directory](https://github.com/lindera/lindera/tree/main/lindera/examples) on GitHub.

The `tokenize*` examples use the `Tokenizer` and filter APIs, which are gated
behind the `analysis` feature (not enabled by default as of v5.0).

Before running the examples, download a pre-built IPADIC dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and extract it to a local directory.

## Available Examples

### segment

Basic morphological segmentation with the `Segmenter` API — works with the
default features (no `analysis` feature required).

```shell
cargo run --features=embed-ipadic --example=segment
```

### tokenize

Basic tokenization using an external IPADIC dictionary. Segments input text and prints each token with its part-of-speech details.

```shell
cargo run --features=embed-ipadic,analysis --example=tokenize
```

### tokenize_with_user_dict

Tokenization with a user dictionary. Shows how to supplement the dictionary with custom entries for domain-specific terms.

```shell
cargo run --features=embed-ipadic,analysis --example=tokenize_with_user_dict
```

### tokenize_with_filters

Tokenization with character filters and token filters. Demonstrates the text processing pipeline, including Unicode normalization, part-of-speech filtering, and other transformations.

```shell
cargo run --features=embed-ipadic,analysis --example=tokenize_with_filters
```

### tokenize_with_config

Tokenization using a YAML configuration file. Shows how to configure the tokenizer declaratively instead of programmatically.

```shell
cargo run --features=embed-ipadic,analysis --example=tokenize_with_config
```

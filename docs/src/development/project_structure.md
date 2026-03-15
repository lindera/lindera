# Project Structure

Lindera is organized as a Cargo workspace with multiple crates.

## Directory Layout

```text
lindera/
├── lindera-crf/            # CRF engine (pure Rust, no_std)
├── lindera-dictionary/     # Dictionary base library
├── lindera/                # Core morphological analysis library
├── lindera-cli/            # CLI tool
├── lindera-ipadic/         # IPADIC dictionary (Japanese)
├── lindera-ipadic-neologd/ # IPADIC NEologd dictionary (Japanese)
├── lindera-unidic/         # UniDic dictionary (Japanese)
├── lindera-ko-dic/         # ko-dic dictionary (Korean)
├── lindera-cc-cedict/      # CC-CEDICT dictionary (Chinese)
├── lindera-jieba/          # Jieba dictionary (Chinese)
├── lindera-python/         # Python bindings (PyO3)
├── lindera-wasm/           # WebAssembly bindings (wasm-bindgen)
├── resources/              # Test resources and sample data
├── docs/                   # Documentation (mdBook)
└── examples/               # Example code
```

## Crate Descriptions

### Core Crates

#### `lindera-crf`

Pure Rust implementation of Conditional Random Fields (CRF). Supports `no_std` environments. Uses `rkyv` for fast zero-copy serialization. This crate provides the statistical learning engine used in dictionary training.

#### `lindera-dictionary`

Base library for dictionary handling: loading, building, and querying dictionaries. With the `train` feature enabled, it also provides the CRF training pipeline for creating custom dictionaries.

Key modules under `src/trainer/`:

| Module | Role |
| --- | --- |
| `config.rs` | Configuration management (seed dict, char.def, feature.def, rewrite.def) |
| `corpus.rs` | Training corpus processing |
| `feature_extractor.rs` | Feature template parsing and feature ID management |
| `feature_rewriter.rs` | MeCab-compatible feature rewriting (3-section format) |
| `model.rs` | Trained model storage, serialization, and dictionary output |

#### `lindera`

The main morphological analysis library. Integrates dictionary crates and provides the `Tokenizer`, `Segmenter`, character filters, and token filters.

#### `lindera-cli`

Command-line interface for tokenization, dictionary training, export, and building. The `train` feature is enabled by default.

### Dictionary Crates

Each dictionary crate contains pre-built dictionary data for a specific language and dictionary source.

| Crate | Language | Dictionary Source |
| --- | --- | --- |
| `lindera-ipadic` | Japanese | IPADIC |
| `lindera-ipadic-neologd` | Japanese | IPADIC NEologd (extended vocabulary) |
| `lindera-unidic` | Japanese | UniDic |
| `lindera-ko-dic` | Korean | ko-dic |
| `lindera-cc-cedict` | Chinese | CC-CEDICT |
| `lindera-jieba` | Chinese | Jieba |

### Bindings

#### `lindera-python`

Python bindings built with [PyO3](https://pyo3.rs/). Exposes the Lindera tokenizer API to Python applications.

#### `lindera-wasm`

WebAssembly bindings built with [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/). Enables tokenization in browsers and Node.js.

### Other Directories

#### `resources/`

Test resources including sample dictionaries, user dictionaries, and test corpora used by the test suite.

#### `docs/`

User-facing documentation built with [mdBook](https://rust-lang.github.io/mdBook/). The table of contents is defined in `docs/src/SUMMARY.md`. A Japanese translation is available under `docs/ja/`.

#### `examples/`

Runnable example programs demonstrating common usage patterns. Run with:

```bash
cargo run --features=embed-ipadic --example=<example_name>
```

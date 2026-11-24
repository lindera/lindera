# Project Structure

Lindera is organized as a Cargo workspace with multiple crates.

## Directory Layout

```text
lindera/
├── lindera-crf/            # CRF engine (pure Rust, no_std)
├── lindera-dictionary/     # Dictionary base library
├── lindera-trainer/        # CRF-based dictionary training
├── lindera/                # Facade crate (re-exports lindera-segmenter and lindera-analysis); tests, examples, benches
├── lindera-segmenter/      # Morphological segmentation library (Segmenter API)
├── lindera-analysis/       # Analysis chain (character/token filters, tokenizer)
├── lindera-dependency/     # Bunsetsu dependency analysis (under development, not published)
├── lindera-cli/            # CLI tool
├── lindera-binding/        # FFI-independent helpers shared by the language bindings
├── lindera-ipadic/         # IPADIC dictionary (Japanese)
├── lindera-ipadic-neologd/ # IPADIC NEologd dictionary (Japanese)
├── lindera-unidic/         # UniDic dictionary (Japanese)
├── lindera-unidic-cwj/     # UniDic CWJ dictionary (Japanese)
├── lindera-unidic-csj/     # UniDic CSJ dictionary (Japanese)
├── lindera-sudachidict/    # SudachiDict dictionary (Japanese)
├── lindera-ko-dic/         # ko-dic dictionary (Korean)
├── lindera-cc-cedict/      # CC-CEDICT dictionary (Chinese)
├── lindera-jieba/          # Jieba dictionary (Chinese)
├── lindera-python/         # Python bindings (PyO3)
├── lindera-nodejs/         # Node.js bindings (NAPI-RS)
├── lindera-ruby/           # Ruby bindings (Magnus + rb-sys)
├── lindera-php/            # PHP bindings (ext-php-rs)
├── lindera-wasm/           # WebAssembly bindings (wasm-bindgen)
├── resources/              # Test resources and sample data
└── docs/                   # Documentation (mdBook)
```

## Crate Descriptions

### Core Crates

#### `lindera-crf`

Pure Rust implementation of Conditional Random Fields (CRF). Supports `no_std` environments. Uses `rkyv` for fast zero-copy serialization. This crate provides the statistical learning engine used in dictionary training.

#### `lindera-dictionary`

Base library for dictionary handling: loading, building, and querying dictionaries.

#### `lindera-trainer`

CRF training pipeline for creating custom dictionaries. Builds on `lindera-dictionary` runtime types and the `lindera-crf` engine. Consumed through the `lindera` facade's `train` feature (re-exported as `lindera::dictionary::trainer`).

| Module | Role |
| --- | --- |
| `config.rs` | Configuration management (seed dict, char.def, feature.def, rewrite.def) |
| `corpus.rs` | Training corpus processing |
| `feature_extractor.rs` | Feature template parsing and feature ID management |
| `feature_rewriter.rs` | MeCab-compatible feature rewriting (3-section format) |
| `model.rs` | Trained model storage, serialization, and dictionary output |

#### `lindera-segmenter`

The morphological segmentation library (published as `lindera` up to v6). Integrates the dictionary crates and provides the `Segmenter` API. Its `dictionary` module also re-exports the raw `lindera-dictionary` modules (`builder`, `core`, `loader`, `viterbi`, ...).

#### `lindera`

The facade crate and the single dependency for Rust users. It re-exports every module of `lindera-segmenter` and, with the default `analysis` feature, `lindera-analysis` as `lindera::analysis`. The directory also holds the integration tests, examples, and benches that exercise the public paths.

#### `lindera-analysis`

Lucene-style analysis chain on top of `lindera-segmenter` (used as `lindera::analysis` through the facade): character filters, token filters, and the `Tokenizer` that composes them around a `Segmenter`.

#### `lindera-dependency`

Japanese bunsetsu dependency analysis, under development and not published yet ([#1417](https://github.com/lindera/lindera/issues/1417)). So far it holds the types the analysis is built on (a morpheme trait, bunsetsu and sentences), the reading and writing of the lattice format of CaboCha (`cabocha -f1`), a check of the dependency tree, the schema of a dictionary (which feature column holds what, and the patterns that give the head and the function word of a bunsetsu and recognize symbols; IPADIC is the first one), the chunker, which segments the morphemes of a sentence into bunsetsu with a conditional random field and a model that is a table of weights, the model file of a dictionary, which is read in place, and the parser, which finds the head of each bunsetsu with shift-reduce and a classifier over features and pairs of features. No model is distributed yet: the chunker and the parser are trained with the commands of the crate. Its default build depends on no other crate of the workspace and builds for `wasm32-unknown-unknown` (`make check-lindera-dependency`). With the `train` feature it also has the `lindera-dependency convert` command, which makes training data from a dependency corpus such as UD Japanese-GSD by tokenizing it with `lindera-segmenter` and reports how often the head of the schema is the content head of the corpus, and the `lindera-dependency evaluate` and `lindera-dependency split` commands, which count the accuracy of chunking and of dependency as CaboCha and J.DepP do and write the folds of a cross-validation, and the `lindera-dependency train-chunker` and `lindera-dependency chunk` commands, which train the model of the chunker with `lindera-crf` and segment a file with it, and the `lindera-dependency instances` command, which writes the training instances of the parser (see the README of the crate). How to use it is in the [Lindera Dependency](../lindera-dependency.md) chapter. The decisions behind the crate and the numbers they rest on are in the [design notes](./dependency_design.md); the scripts that give the numbers are in `scripts/dependency/`.

#### `lindera-cli`

Command-line interface for tokenization, dictionary training, export, and building. The `train` feature is enabled by default.

#### `lindera-binding`

FFI-independent helpers shared by all five language bindings (`lindera-python`, `lindera-nodejs`, `lindera-ruby`, `lindera-php`, `lindera-wasm`): a core tokenizer/schema/metadata layer that each binding wraps in its own language-native API. It depends on the `lindera` facade.

### Dictionary Crates

Each dictionary crate contains pre-built dictionary data for a specific language and dictionary source.

| Crate | Language | Dictionary Source |
| --- | --- | --- |
| `lindera-ipadic` | Japanese | IPADIC |
| `lindera-ipadic-neologd` | Japanese | IPADIC NEologd (extended vocabulary) |
| `lindera-unidic` | Japanese | UniDic |
| `lindera-unidic-cwj` | Japanese | UniDic CWJ |
| `lindera-unidic-csj` | Japanese | UniDic CSJ |
| `lindera-sudachidict` | Japanese | SudachiDict |
| `lindera-ko-dic` | Korean | ko-dic |
| `lindera-cc-cedict` | Chinese | CC-CEDICT |
| `lindera-jieba` | Chinese | Jieba |

### Bindings

#### `lindera-python`

Python bindings built with [PyO3](https://pyo3.rs/). Exposes the Lindera tokenizer API to Python applications.

#### `lindera-nodejs`

Node.js bindings built with [NAPI-RS](https://napi.rs/). Exposes the Lindera tokenizer API to Node.js applications.

#### `lindera-ruby`

Ruby bindings built with [Magnus](https://github.com/matsadler/magnus) and `rb-sys`. Exposes the Lindera tokenizer API as a Ruby gem.

#### `lindera-php`

PHP bindings built with [ext-php-rs](https://github.com/davidcole1340/ext-php-rs). Exposes the Lindera tokenizer API as a PHP extension. The extension registers as `lindera` and is published on Packagist as `lindera/lindera` for [PIE](https://github.com/php/pie): the package's `composer.json` lives at the repository root and points PIE at `lindera-php/`, where `config.m4` and `Makefile.frag` drive `cargo build` through the phpize build.

#### `lindera-wasm`

WebAssembly bindings built with [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/). Enables tokenization in browsers and Node.js.

### Other Directories

#### `resources/`

Test resources including sample dictionaries, user dictionaries, and test corpora used by the test suite.

#### `docs/`

User-facing documentation built with [mdBook](https://rust-lang.github.io/mdBook/). The table of contents is defined in `docs/src/SUMMARY.md`. A Japanese translation is available under `docs/ja/`.

#### `lindera/examples/` and `lindera-analysis/examples/`

Runnable example programs demonstrating common usage patterns. Run with:

```bash
cargo run -p lindera --features=embed-ipadic --example=segment
cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize
```

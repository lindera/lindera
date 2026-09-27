# Architecture

Lindera is organized as a Cargo workspace comprising multiple crates. Each crate has a focused responsibility, from low-level CRF computation to high-level CLI and language bindings.

## Crate Dependency Graph

```mermaid
graph TB
    CRF["lindera-crf\n(CRF Engine)"]
    DICT["lindera-dictionary\n(Dictionary Base)"]
    TRAINER["lindera-trainer\n(CRF Training)"]
    IPADIC["lindera-ipadic"]
    UNIDIC["lindera-unidic"]
    SUDACHIDICT["lindera-sudachidict"]
    KODIC["lindera-ko-dic"]
    CCCEDICT["lindera-cc-cedict"]
    JIEBA["lindera-jieba"]
    NEOLOGD["lindera-ipadic-neologd"]
    SEGMENTER["lindera-segmenter\n(Segmenter)"]
    ANALYSIS["lindera-analysis\n(Analysis Chain)"]
    LIB["lindera\n(Facade)"]
    CLI["lindera-cli\n(CLI)"]
    BINDING["lindera-binding"]
    PY["lindera-python"]
    NODEJS["lindera-nodejs"]
    RUBY["lindera-ruby"]
    PHP["lindera-php"]
    WASM["lindera-wasm"]

    CRF --> TRAINER
    DICT --> TRAINER
    TRAINER -.->|"train feature"| SEGMENTER
    DICT --> IPADIC
    DICT --> UNIDIC
    DICT --> SUDACHIDICT
    DICT --> KODIC
    DICT --> CCCEDICT
    DICT --> JIEBA
    DICT --> NEOLOGD
    DICT --> SEGMENTER
    IPADIC --> SEGMENTER
    UNIDIC --> SEGMENTER
    SUDACHIDICT --> SEGMENTER
    KODIC --> SEGMENTER
    CCCEDICT --> SEGMENTER
    JIEBA --> SEGMENTER
    NEOLOGD --> SEGMENTER
    SEGMENTER --> ANALYSIS
    SEGMENTER --> LIB
    ANALYSIS -.->|"analysis feature"| LIB
    LIB --> CLI
    LIB --> BINDING
    BINDING --> PY
    BINDING --> NODEJS
    BINDING --> RUBY
    BINDING --> PHP
    BINDING --> WASM
```

## Crate Overview

| Crate | Type | Description |
| --- | --- | --- |
| `lindera-crf` | Core | Pure Rust CRF (Conditional Random Field) implementation. Supports `no_std`. Uses `rkyv` for serialization. |
| `lindera-dictionary` | Core | Dictionary base library. Provides dictionary loading and building. |
| `lindera-trainer` | Core | CRF-based dictionary training pipeline. Builds on `lindera-crf` and `lindera-dictionary`; consumed directly or via the `lindera` facade's `train` feature. |
| `lindera-segmenter` | Core | Pure morphological segmenter (the `lindera` crate of v6 and earlier). Integrates the dictionary crates and provides the `Segmenter` API. |
| `lindera-analysis` | Core | Lucene-style analysis chain on top of `lindera-segmenter`: character filters, token filters, and the `Tokenizer` that composes them around a `Segmenter`. Reachable as `lindera::analysis`. |
| `lindera` | Core | Facade crate and the one dependency Rust users add: re-exports `lindera-segmenter` and, with the default `analysis` feature, `lindera-analysis` as `lindera::analysis`. |
| `lindera-cli` | Application | Command-line interface for tokenization, dictionary building, and CRF training. |
| `lindera-binding` | Core | FFI-independent helpers shared by the five language bindings below; depends on the `lindera` facade. |
| `lindera-ipadic` | Dictionary | Japanese dictionary based on IPADIC. |
| `lindera-ipadic-neologd` | Dictionary | Japanese dictionary based on IPADIC NEologd (includes neologisms). |
| `lindera-unidic` | Dictionary | Japanese dictionary based on UniDic. |
| `lindera-sudachidict` | Dictionary | Japanese dictionary based on SudachiDict. |
| `lindera-ko-dic` | Dictionary | Korean dictionary based on ko-dic. |
| `lindera-cc-cedict` | Dictionary | Chinese dictionary based on CC-CEDICT. |
| `lindera-jieba` | Dictionary | Chinese dictionary based on Jieba. |
| `lindera-python` | Binding | Python bindings via PyO3. |
| `lindera-nodejs` | Binding | Node.js bindings via NAPI-RS. |
| `lindera-ruby` | Binding | Ruby bindings via Magnus + rb-sys. |
| `lindera-php` | Binding | PHP bindings via ext-php-rs. |
| `lindera-wasm` | Binding | WebAssembly bindings via wasm-bindgen. |

## Tokenization Pipeline

Lindera processes text through a multi-stage pipeline:

```text
Input Text
  |
  v
Character Filters    -- Normalize characters (e.g., Unicode normalization, mapping)
  |
  v
Segmenter            -- Segment text into tokens using a dictionary and the Viterbi algorithm
  |
  v
Token Filters        -- Transform tokens (e.g., POS filtering, stop words, stemming)
  |
  v
Output Tokens
```

The **Segmenter** is the core component. It builds a lattice of candidate tokens from the dictionary, then applies the Viterbi algorithm to find the lowest-cost path, producing the most likely segmentation. Dictionary lookups walk a char-wise double-array trie (built with `crawdad`) in place over the serialized bytes of `dict.trie`, rather than deserializing into an owned structure. For repeated tokenization, `Segmenter::new_worker()` returns a `SegmentWorker` that reuses lattice and scratch-buffer allocations across calls, with an automatic shrink policy to bound retained memory.

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `analysis` | Re-export of `lindera-analysis` as `lindera::analysis` (character filters, token filters, `Tokenizer`) | Enabled (off with `default-features = false`) |
| `mmap` | Memory-mapped file support for filesystem-based dictionary loading (opt-in via `--mmap`/`use_mmap`; the word-list files, the connection-cost matrix, and the trie all stay lazily paged) | Enabled |
| `train` | CRF-based dictionary training functionality (depends on `lindera-crf`) | CLI + Python/Node.js/Ruby/PHP bindings (default); opt-in for the `lindera` library crate; unavailable in `lindera-wasm` |
| `embed-ipadic` | Embed the IPADIC dictionary into the binary | Disabled |
| `embed-ipadic-neologd` | Embed the IPADIC NEologd dictionary into the binary | Disabled |
| `embed-unidic` | Embed the UniDic dictionary into the binary | Disabled |
| `embed-sudachidict` | Embed the SudachiDict dictionary into the binary | Disabled |
| `embed-ko-dic` | Embed the ko-dic dictionary into the binary | Disabled |
| `embed-cc-cedict` | Embed the CC-CEDICT dictionary into the binary | Disabled |
| `embed-jieba` | Embed the Jieba dictionary into the binary | Disabled |
| `embed-cjk` | Embed IPADIC + ko-dic + Jieba dictionaries | Disabled |
| `embed-cjk2` | Embed UniDic + ko-dic + Jieba dictionaries | Disabled |
| `embed-cjk3` | Embed IPADIC NEologd + ko-dic + Jieba dictionaries | Disabled |
| `embed-cjk4` | Embed SudachiDict + ko-dic + Jieba dictionaries | Disabled |

## Learn More

- [Getting Started](./getting_started.md) -- Installation and first steps
- [Core Concepts](./concepts.md) -- Dictionaries, tokenization, and filters
- [Lindera Library](./lindera.md) -- The `lindera` facade and the segmenter API
- [Lindera Analysis](./lindera-analysis.md) -- Character filters, token filters, and the `Tokenizer`
- [Lindera Dictionary](./lindera-dictionary.md) -- Dictionary loading and building
- [Lindera Trainer](./lindera-trainer.md) -- CRF-based dictionary training
- [Lindera CRF](./lindera-crf.md) -- The CRF engine
- [Lindera CLI](./lindera-cli.md) -- Command-line interface
- [Development Guide](./development.md) -- Build, test, and contribute

# Architecture

Lindera is organized as a Cargo workspace comprising multiple crates. Each crate has a focused responsibility, from low-level CRF computation to high-level CLI and language bindings.

## Crate Dependency Graph

```mermaid
graph TB
    CRF["lindera-crf\n(CRF Engine)"]
    DICT["lindera-dictionary\n(Dictionary Base)"]
    IPADIC["lindera-ipadic"]
    UNIDIC["lindera-unidic"]
    KODIC["lindera-ko-dic"]
    CCCEDICT["lindera-cc-cedict"]
    JIEBA["lindera-jieba"]
    NEOLOGD["lindera-ipadic-neologd"]
    LIB["lindera\n(Core Library)"]
    CLI["lindera-cli\n(CLI)"]
    PY["lindera-python\n(Python)"]
    WASM["lindera-wasm\n(WebAssembly)"]

    CRF --> DICT
    DICT --> IPADIC
    DICT --> UNIDIC
    DICT --> KODIC
    DICT --> CCCEDICT
    DICT --> JIEBA
    DICT --> NEOLOGD
    DICT --> LIB
    IPADIC --> LIB
    UNIDIC --> LIB
    KODIC --> LIB
    CCCEDICT --> LIB
    JIEBA --> LIB
    NEOLOGD --> LIB
    LIB --> CLI
    LIB --> PY
    LIB --> WASM
```

## Crate Overview

| Crate | Type | Description |
| --- | --- | --- |
| `lindera-crf` | Core | Pure Rust CRF (Conditional Random Field) implementation. Supports `no_std`. Uses `rkyv` for serialization. |
| `lindera-dictionary` | Core | Dictionary base library. Provides dictionary loading, building, and training (with the `train` feature). |
| `lindera` | Core | Main morphological analysis library. Integrates dictionaries, segmenter, character filters, and token filters. |
| `lindera-cli` | Application | Command-line interface for tokenization, dictionary building, and CRF training. |
| `lindera-ipadic` | Dictionary | Japanese dictionary based on IPADIC. |
| `lindera-ipadic-neologd` | Dictionary | Japanese dictionary based on IPADIC NEologd (includes neologisms). |
| `lindera-unidic` | Dictionary | Japanese dictionary based on UniDic. |
| `lindera-ko-dic` | Dictionary | Korean dictionary based on ko-dic. |
| `lindera-cc-cedict` | Dictionary | Chinese dictionary based on CC-CEDICT. |
| `lindera-jieba` | Dictionary | Chinese dictionary based on Jieba. |
| `lindera-python` | Binding | Python bindings via PyO3. |
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

The **Segmenter** is the core component. It builds a lattice of candidate tokens from the dictionary, then applies the Viterbi algorithm to find the lowest-cost path, producing the most likely segmentation.

## Feature Flags

| Feature | Description | Default |
| --- | --- | --- |
| `compress` | Dictionary compression support | Enabled |
| `mmap` | Memory-mapped file support for dictionary loading | Enabled |
| `train` | CRF-based dictionary training functionality (depends on `lindera-crf`) | CLI only |
| `embed-ipadic` | Embed the IPADIC dictionary into the binary | Disabled |
| `embed-cjk` | Embed IPADIC + ko-dic + Jieba dictionaries | Disabled |
| `embed-cjk2` | Embed UniDic + ko-dic + Jieba dictionaries | Disabled |
| `embed-cjk3` | Embed IPADIC NEologd + ko-dic + Jieba dictionaries | Disabled |

## Learn More

- [Getting Started](./getting_started.md) -- Installation and first steps
- [Core Concepts](./concepts.md) -- Dictionaries, tokenization, and filters
- [Lindera Library](./lindera.md) -- Configuration, segmenter, and API
- [Lindera CLI](./lindera-cli.md) -- Command-line interface
- [Development Guide](./development.md) -- Build, test, and contribute

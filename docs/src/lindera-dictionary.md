# Lindera Dictionary

Lindera Dictionary is the base library for morphological analysis dictionaries. It provides dictionary loading, building, and Viterbi-based segmentation. CRF-based dictionary training lives in the separate [`lindera-trainer`](./lindera-trainer.md) crate. Through the `lindera` facade its modules are available under `lindera::dictionary` — `lindera::dictionary::core` (the dictionary data structures), `lindera::dictionary::builder`, `lindera::dictionary::viterbi`, and so on — so a direct `lindera-dictionary` dependency is rarely needed.

## Key Features

- Dictionary loading from filesystem or embedded data
- Dictionary building from MeCab-format CSV source files
- Viterbi algorithm for optimal segmentation
- N-best path generation (Forward-DP Backward-A*)
- Memory-mapped file support

## Contents

- [Architecture](./lindera-dictionary/architecture.md) -- Internal structure and key components
- [API Reference](./lindera-dictionary/api_reference.md) -- API documentation

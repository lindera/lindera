# Lindera Trainer

Lindera Trainer implements the CRF-based dictionary training pipeline (`lindera train`), turning an annotated corpus and a seed lexicon into a trained model that can be exported to MeCab-format dictionary source files and compiled into a binary dictionary. It builds on the runtime types of `lindera-dictionary` and the CRF core of `lindera-crf`. The crate can be used directly, or through the `lindera` facade's `train` feature, which re-exports it as `lindera::dictionary::trainer`.

## Key Features

- CRF-based weight learning via `lindera-crf`, with L1, L2, and Elastic Net regularization
- MeCab-compatible feature template parsing (`feature.def`: `%F[n]`, `%L[n]`, `%R[n]`, `%w`, `%u`, `%l`, `%r`, and their optional `?` variants)
- MeCab-compatible 3-section feature rewriting (`rewrite.def`: unigram / left / right rewrite rules)
- Dictionary-format agnostic: works with any lexicon whose columns follow `surface,left_id,right_id,cost,feature...` (IPADIC, UniDic, ko-dic, CC-CEDICT, etc.)
- Automatic unknown-word categorization driven by `char.def` character categories
- Connection cost matrix and MeCab-compatible cost conversion (`tocost`) derived from learned CRF weights
- Zero-copy `rkyv` binary model serialization, with legacy JSON fallback on read
- Direct export of Lindera/MeCab dictionary source files (`lex.csv`, `matrix.def`, `unk.def`, `char.def`, `feature.def`, `rewrite.def`, `left-id.def`, `right-id.def`)

## Contents

- [Architecture](./lindera-trainer/architecture.md) -- Internal structure and key components
- [API Reference](./lindera-trainer/api_reference.md) -- API documentation

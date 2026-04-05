# Architecture

## Module Structure

```text
lindera-dictionary/src/
├── lib.rs               # Public API
├── dictionary.rs        # Dictionary, UserDictionary
├── builder.rs           # DictionaryBuilder
├── loader.rs            # DictionaryLoader trait, FSDictionaryLoader
├── viterbi.rs           # Lattice, Edge, Viterbi segmentation
├── nbest.rs             # NBestGenerator (Forward-DP Backward-A*)
├── mode.rs              # Mode (Normal/Decompose), Penalty
├── error.rs             # LinderaError, LinderaErrorKind
├── assets.rs            # Download and file management
├── dictionary/
│   ├── character_definition.rs    # Character type definitions
│   ├── connection_cost_matrix.rs  # Connection cost matrix
│   ├── prefix_dictionary.rs       # Double-array trie dictionary
│   ├── unknown_dictionary.rs      # Unknown word handling
│   ├── metadata.rs                # Dictionary metadata
│   └── schema.rs                  # Schema definitions
└── trainer/             # (train feature)
    ├── config.rs        # TrainerConfig
    ├── corpus.rs        # Corpus, Example, Word
    ├── feature_extractor.rs  # Feature template parsing
    ├── feature_rewriter.rs   # MeCab-compatible rewrite rules
    └── model.rs         # Trained model, tocost()
```

## Key Components

### Dictionary / UserDictionary

Main data structures holding the compiled dictionary data. A `Dictionary` contains the character definitions, connection cost matrix, prefix dictionary (double-array trie), and unknown word dictionary. `UserDictionary` allows users to add custom vocabulary on top of the system dictionary.

### DictionaryBuilder

Fluent API for building dictionaries from source CSV files. It compiles MeCab-format dictionary sources into the binary format used at runtime.

### DictionaryLoader / FSDictionaryLoader

`DictionaryLoader` is a trait for loading compiled dictionaries. `FSDictionaryLoader` is the filesystem-based implementation that reads dictionary files from a directory, with optional memory-mapped file support.

### Viterbi (Lattice, Edge)

Builds a lattice of candidate tokens from the input text and finds the optimal segmentation path using the Viterbi algorithm. Each `Edge` in the lattice represents a candidate token with associated costs (word cost + connection cost).

### NBestGenerator

Generates N-best segmentation paths using the Forward-DP Backward-A* algorithm. This enables applications to consider alternative segmentations beyond the single best path.

### Mode

Controls tokenization behavior:

- **Normal**: Standard tokenization using the optimal Viterbi path
- **Decompose**: Further splits compound nouns based on configurable `Penalty` thresholds

### Trainer (train feature)

CRF-based dictionary training pipeline using `lindera-crf`. The training workflow includes:

1. **TrainerConfig**: Parses seed dictionary, `char.def`, `feature.def`, and `rewrite.def`
2. **Corpus**: Manages training data as labeled examples
3. **FeatureExtractor**: Parses feature templates and assigns feature IDs
4. **DictionaryRewriter**: Applies MeCab-compatible 3-section rewrite rules
5. **Model**: Holds training results and exports dictionary files with cost conversion via `tocost(weight, cost_factor)`

## Feature Flags

| Feature | Description | Default |
| --------- | ------------- | --------- |
| `mmap` | Memory-mapped file support | Yes |
| `build_rs` | HTTP download for dictionary sources | No |
| `train` | CRF-based training (depends on lindera-crf) | No |

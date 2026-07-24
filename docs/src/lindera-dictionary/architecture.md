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
├── macros.rs            # embedded_dictionary! macro shared by dictionary crates
├── dictionary/
│   ├── character_definition.rs    # Character type definitions
│   ├── connection_cost_matrix.rs  # Connection cost matrix
│   ├── context_id_map.rs          # ContextIdMap: connection-cost context-ID remap
│   ├── prefix_dictionary.rs       # Double-array trie dictionary
│   ├── unknown_dictionary.rs      # Unknown word handling
│   ├── metadata.rs                # Dictionary metadata
│   └── schema.rs                  # Schema definitions
```

## Key Components

### Dictionary / UserDictionary

Main data structures holding the compiled dictionary data. A `Dictionary` contains the character definitions, connection cost matrix, prefix dictionary (double-array trie), and unknown word dictionary. `UserDictionary` allows users to add custom vocabulary on top of the system dictionary.

### DictionaryBuilder

Fluent API for building dictionaries from source CSV files. It compiles MeCab-format dictionary sources into the binary format used at runtime. Its four build stages (metadata, unknown dictionary, prefix dictionary, connection cost matrix) run concurrently on scoped threads on non-wasm targets, falling back to a sequential build on wasm (which has no OS threads); the concurrent path has higher peak memory, since all four stages' working sets are held at once.

### DictionaryLoader / FSDictionaryLoader

`DictionaryLoader` is a trait for loading compiled dictionaries. `FSDictionaryLoader` is the filesystem-based implementation that reads dictionary files from a directory, with optional memory-mapped file support.

### Embedded Dictionary Macro

The `embedded_dictionary!` macro (`lindera-dictionary/src/macros.rs`, `#[macro_export]`) generates the boilerplate each dictionary crate needs to bake its compiled dictionary into the binary: a `load()` function that reads the dictionary components via `include_bytes!` and a loader struct implementing `DictionaryLoader`. Every dictionary crate's `embedded.rs` (`lindera-ipadic`, `lindera-ipadic-neologd`, `lindera-unidic`, `lindera-ko-dic`, `lindera-cc-cedict`, `lindera-jieba`) invokes this macro instead of duplicating the loading logic.

### Viterbi (Lattice, Edge)

Builds a lattice of candidate tokens from the input text and finds the optimal segmentation path using the Viterbi algorithm. Each `Edge` in the lattice represents a candidate token with associated costs (word cost + connection cost).

### NBestGenerator

Generates N-best segmentation paths using the Forward-DP Backward-A* algorithm. This enables applications to consider alternative segmentations beyond the single best path.

### Mode

Controls tokenization behavior:

- **Normal**: Standard tokenization using the optimal Viterbi path
- **Decompose**: Further splits compound nouns based on configurable `Penalty` thresholds

### Context ID Remapping

Dictionary metadata (`lindera-dictionary/src/dictionary/metadata.rs`) carries a `connection_id_mapping: bool` flag and an optional `context_id_map: Option<ContextIdMap>`. When a dictionary crate (e.g. `lindera-unidic`) enables `connection_id_mapping`, `DictionaryBuilder` relabels the connection matrix's left/right context IDs by access frequency at build time, so that frequently-used connection-cost cells cluster together for better cache locality; `DictionaryBuilder::with_context_id_freq` attaches an optional bundled frequency histogram used to rank the IDs, and the remap itself is computed in `lindera-dictionary/src/builder/context_id_remap.rs`. `ContextIdMap` (`lindera-dictionary/src/dictionary/context_id_map.rs`) holds the resulting `left`/`right` permutation and is persisted into the built `metadata.json` so the same mapping can be reapplied later. Because user dictionaries are always compiled in the original, un-remapped ID space, `UserDictionary::remap_context_ids` (`lindera-dictionary/src/dictionary.rs`) uses the persisted `ContextIdMap` to relabel a user dictionary's context IDs into the same space as the system dictionary it is attached to. The remap is a bijective relabeling, so it does not change tokenization output, only lookup locality.

### Training

The CRF-based dictionary training pipeline lives in the separate `lindera-trainer` crate, which builds on this crate's runtime types. See the training pipeline documentation for details.

## Feature Flags

| Feature | Description | Default |
| --------- | ------------- | --------- |
| `mmap` | Memory-mapped file support | Yes |
| `build_rs` | HTTP download for dictionary sources | No |
| `ctxfreq` | Experimental: instruments connection-matrix access-frequency profiling, used to build the context-ID frequency remap | No |

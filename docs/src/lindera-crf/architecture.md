# Architecture

## Module Structure

```text
lindera-crf/src/
├── lib.rs                # Public API re-exports
├── feature.rs            # FeatureSet, FeatureProvider
├── lattice.rs            # Edge, Node, Lattice
├── model.rs              # RawModel, MergedModel, Model trait
├── trainer.rs            # Trainer, Regularization enum
├── errors.rs             # Error types
├── forward_backward.rs   # Forward-backward algorithm
├── math.rs               # Mathematical utilities (logsumexp)
├── optimizers/
│   └── lbfgs.rs          # L-BFGS optimization
└── utils.rs              # Utility traits
```

## Key Components

### FeatureProvider / FeatureSet

Manage per-label feature sets. Each `FeatureSet` holds unigram features and left/right bigram features for a given label. `FeatureProvider` aggregates `FeatureSet` instances and maps feature IDs to weights.

### Lattice / Edge / Node

Lattice structure with variable-length edges for sequence labeling. `Edge` represents a candidate span with a label, while `Node` aggregates edges at a given position. The `Lattice` is constructed from input data and used by the model to find the best path.

### Trainer

Trains a CRF model using L-BFGS optimization with configurable regularization. The trainer accepts labeled lattice examples, computes gradients via the forward-backward algorithm, and iteratively updates model weights.

### Regularization

Configurable regularization strategies:

- **L1**: Sparse models via L1 penalty
- **L2**: Smooth models via L2 penalty
- **ElasticNet**: Combines L1 and L2 with a configurable `l1_ratio`

### Model (trait)

Interface for searching the best path through a lattice. Two implementations are provided:

- **RawModel**: Stores weights in a flat vector indexed by feature ID
- **MergedModel**: Optimized for inference; merges feature weights into a compact representation serializable with rkyv

### Forward-backward Algorithm

Computes alpha (forward) and beta (backward) values over the lattice. Used during training to calculate expected feature counts and gradients.

## Feature Flags

| Feature | Description | Default |
| --------- | ------------- | --------- |
| `alloc` | Alloc support for `no_std` | No |
| `std` | Standard library support (implies `alloc`) | No |
| `train` | Training functionality (L-BFGS, multi-threading, logging) | Yes |

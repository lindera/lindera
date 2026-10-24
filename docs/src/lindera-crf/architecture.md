# Architecture

## Module Structure

```text
lindera-crf/src/
├── lib.rs                # Public API re-exports
├── bigram_table.rs       # Bigram weight indices of the training lattices
├── feature.rs            # FeatureSet, FeatureProvider
├── lattice.rs            # Edge, Node, Lattice
├── model.rs              # RawModel, MergedModel, Model trait
├── trainer.rs            # Trainer, Regularization, StopReason, TrainingStats
├── errors.rs             # Error types
├── forward_backward.rs   # Forward-backward algorithm
├── math.rs               # Mathematical utilities (logsumexp)
├── optimizers.rs         # Declares the optimizers module
├── optimizers/
│   └── lbfgs.rs          # L-BFGS optimization
└── utils.rs              # Utility traits
```

## Key Components

### FeatureProvider / FeatureSet

Manage per-label feature sets. Each `FeatureSet` holds unigram features and left/right bigram features for a given label (feature IDs only, no weights). `FeatureProvider` maps label IDs to `FeatureSet` instances. Weights are held separately, on `RawModel` (`weights`, `unigram_weight_indices`, `bigram_weight_indices`).

### Lattice / Edge / Node

Lattice structure with variable-length edges for sequence labeling. `Edge` represents a candidate span with a label, while `Node` aggregates edges at a given position. The `Lattice` is constructed from input data and used by the model to find the best path.

### Trainer

Trains a CRF model using L-BFGS optimization (OWL-QN when the regularization has an L1 part) with configurable regularization. The trainer accepts labeled lattice examples, computes the loss and its gradient via the forward-backward algorithm, and iteratively updates model weights. `Trainer::train` returns the trained `RawModel`; `Trainer::train_with_stats` also returns a `TrainingStats` with:

- `iterations`: the number of iterations that ran, which is the maximum set with `Trainer::max_iter` when that stopped training, and fewer when the optimizer stopped earlier.
- `evaluations`: the number of evaluations of the loss and its gradient, each one forward-backward pass over every lattice, line-search trials included.
- `stop_reason`: why the optimizer stopped, a `StopReason`.

The optimizer stops at the first of these:

- `StopReason::Converged`: the relative change of the objective, `|previous - current| / |previous|`, stayed below the tolerance set with `Trainer::eta` for 3 iterations in a row, or the gradient vanished. The default tolerance, `DEFAULT_ETA` (`5e-5`), is the default `eta` of MeCab's learner, which applies the same test to every evaluation, line-search trials included, where Lindera counts iterations. `Trainer::eta(0.0)` turns the test off.
- `StopReason::NoProgress`: an iteration left the objective unchanged. When this happens after a few iterations, the model is barely trained.
- `StopReason::MaxIterations`: the cap set with `Trainer::max_iter`.
- `StopReason::LineSearchFailed`: the line search of an iteration failed. The weights are the best ones of the run.

### Regularization

Configurable regularization strategies:

- **L1**: Sparse models via L1 penalty
- **L2**: Smooth models via L2 penalty
- **ElasticNet**: Combines L1 and L2 with a configurable `l1_ratio`

`Trainer::new` uses L2 with `DEFAULT_LAMBDA` (1.0): the objective of MeCab's learner with its default `-c 1.0`. MeCab adds `‖w‖² / (2C)`, so the coefficient is `1 / C` in general.

### Model (trait)

Interface for searching the best path through a lattice. Two implementations are provided:

- **RawModel**: Stores weights in a flat vector indexed by feature ID
- **MergedModel**: Optimized for inference; merges feature weights into a compact representation serializable with rkyv, using `MergedFeatureSet` as its per-label element type

`MergedFeatureSet` holds the pre-summed unigram `weight` for a label, along with its `left_id`/`right_id` bigram connection IDs into `MergedModel`'s bigram weight matrix.

`RawModel::unigram_weight` computes that sum for a list of unigram feature IDs, and `merge` calls it for every label. A feature without a weight adds nothing, so it also weighs the features of an entry that was not a label in training: `lindera-trainer` uses it for the cost of a user lexicon entry.

### Forward-backward Algorithm

Computes alpha (forward) and beta (backward) values over the lattice. Used during training to calculate the loss and the expected feature counts. One pass over a lattice yields both its loss and its share of the gradient. The optimizer keeps the result for the last parameter vector, because argmin's L-BFGS asks for the cost and the gradient of the same point in two calls.

Before the optimizer starts, a bigram table records, for every pair of edges that meet in the lattices, the weight indices of their bigram features, in the order the evaluation adds them; labels with the same bigram feature IDs share the entries. The evaluations read the indices from the table instead of looking each bigram feature up in a map, and add the same weights in the same order, so the trained weights do not change.

## Feature Flags

| Feature | Description | Default |
| --------- | ------------- | --------- |
| `alloc` | Alloc support for `no_std` | No* |
| `std` | Standard library support (implies `alloc`) | No* |
| `train` | Training functionality (L-BFGS, multi-threading, logging) | Yes |

\* Not listed directly in `default`, but transitively enabled by `train` (which is on by default and enables `std`, which in turn enables `alloc`) -- so `alloc`/`std` are active in a default build.

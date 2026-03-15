# lindera-crf: Conditional Random Fields implemented in pure Rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera-crf.svg)](https://crates.io/crates/lindera-crf)

*lindera-crf* is forked from [rucrf](https://github.com/daac-tools/rucrf) and contains a trainer and an estimator for Conditional Random Fields (CRFs).
This library supports:

- [x] lattices with variable length edges,
- [x] L1, L2, and Elastic Net regularization,
- [x] multi-threading during training, and
- [x] zero-copy deserialization with rkyv.

## Changes from rucrf

- **Serialization backend replaced**: Switched from `bincode` to [`rkyv`](https://github.com/rkyv/rkyv) for zero-copy deserialization, enabling faster model loading
- **Elastic Net regularization**: Added `Regularization::ElasticNet` that combines L1 and L2 penalties with a configurable `l1_ratio` parameter
- **Rust 2024 edition**: Updated to Rust 2024 edition
- **Dependency updates**: Updated `argmin` (0.10 -> 0.11), `argmin-math` (0.4 -> 0.5), `argmin-observer-slog` (0.1 -> 0.2), `hashbrown` (0.15 -> 0.16)

## Examples

```rust
use std::num::NonZeroU32;

use lindera_crf::{Edge, FeatureProvider, FeatureSet, Lattice, Model, Trainer};

// Train:
// 京(kyo) 都(to)
// 東(to) 京(kyo)
// 京(kei) 浜(hin)
// 京(kyo) の(no) 都(miyako)
//
// Test:
// 水(mizu) の(no) 都(miyako)
//
// 1-gram features:
// 京: 1, 都: 2, 東: 3, 浜: 4, の: 5, 水: 6
// 2-gram features:
// kyo: 1, to: 2, kei: 3, hin: 4, no: 5, miyako: 6, mizu: 7

let mut provider = FeatureProvider::new();
let label_京kyo = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(1).unwrap()],
    &[NonZeroU32::new(1)],
    &[NonZeroU32::new(1)],
))?;
let label_都to = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(2).unwrap()],
    &[NonZeroU32::new(2)],
    &[NonZeroU32::new(2)],
))?;
let label_東to = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(3).unwrap()],
    &[NonZeroU32::new(2)],
    &[NonZeroU32::new(2)],
))?;
let label_京kei = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(1).unwrap()],
    &[NonZeroU32::new(3)],
    &[NonZeroU32::new(3)],
))?;
let label_浜hin = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(4).unwrap()],
    &[NonZeroU32::new(4)],
    &[NonZeroU32::new(4)],
))?;
let label_のno = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(5).unwrap()],
    &[NonZeroU32::new(5)],
    &[NonZeroU32::new(5)],
))?;
let label_都miyako = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(2).unwrap()],
    &[NonZeroU32::new(6)],
    &[NonZeroU32::new(6)],
))?;
let label_水mizu = provider.add_feature_set(FeatureSet::new(
    &[NonZeroU32::new(6).unwrap()],
    &[NonZeroU32::new(7)],
    &[NonZeroU32::new(7)],
))?;

let mut lattices = vec![];

// 京都 (kyo to)
let mut lattice = Lattice::new(2)?;
lattice.add_edge(0, Edge::new(1, label_京kyo))?;
lattice.add_edge(1, Edge::new(2, label_都to))?;

lattice.add_edge(0, Edge::new(1, label_京kei))?;
lattice.add_edge(1, Edge::new(2, label_都miyako))?;

lattices.push(lattice);

// 東京 (to kyo)
let mut lattice = Lattice::new(2)?;
lattice.add_edge(0, Edge::new(1, label_東to))?;
lattice.add_edge(1, Edge::new(2, label_京kyo))?;

lattice.add_edge(1, Edge::new(2, label_京kei))?;

lattices.push(lattice);

// 京浜 (kei hin)
let mut lattice = Lattice::new(2)?;
lattice.add_edge(0, Edge::new(1, label_京kei))?;
lattice.add_edge(1, Edge::new(2, label_浜hin))?;

lattice.add_edge(0, Edge::new(1, label_京kyo))?;

lattices.push(lattice);

// 京の都 (kyo no miyako)
let mut lattice = Lattice::new(3)?;
lattice.add_edge(0, Edge::new(1, label_京kyo))?;
lattice.add_edge(1, Edge::new(2, label_のno))?;
lattice.add_edge(2, Edge::new(3, label_都miyako))?;

lattice.add_edge(0, Edge::new(1, label_京kei))?;
lattice.add_edge(2, Edge::new(3, label_都to))?;

lattices.push(lattice);

// Generates a model
let trainer = Trainer::new();
let model = trainer.train(&lattices, provider);

// 水の都 (mizu no miyako)
let mut lattice = Lattice::new(3)?;
lattice.add_edge(0, Edge::new(1, label_水mizu))?;
lattice.add_edge(1, Edge::new(2, label_のno))?;
lattice.add_edge(2, Edge::new(3, label_都to))?;
lattice.add_edge(2, Edge::new(3, label_都miyako))?;

let (path, _) = model.search_best_path(&lattice);

assert_eq!(vec![
    Edge::new(1, label_水mizu),
    Edge::new(2, label_のno),
    Edge::new(3, label_都miyako),
], path);
```

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

See [the guidelines](./CONTRIBUTING.md).

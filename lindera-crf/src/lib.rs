//! # lindera-crf
//!
//! Conditional Random Fields (CRFs) implemented in pure Rust
#![cfg_attr(
    all(feature = "std", feature = "train"),
    doc = "
## Examples

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
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
# Ok(())
# }
```
"
)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "alloc"))]
compile_error!("`alloc` feature is currently required to build this crate");

#[macro_use]
extern crate alloc;

pub mod errors;
mod feature;
mod lattice;
mod model;
mod utils;

#[cfg(feature = "train")]
mod bigram_table;
#[cfg(feature = "train")]
mod forward_backward;
#[cfg(feature = "train")]
mod math;
#[cfg(feature = "train")]
mod optimizers;
#[cfg(feature = "train")]
mod trainer;

#[cfg(test)]
mod test_utils;

pub use feature::{FeatureProvider, FeatureSet};
pub use lattice::{Edge, Lattice};
pub use model::{MergedFeatureSet, MergedModel, Model, RawModel};

#[cfg(feature = "train")]
pub use trainer::{Regularization, Trainer, TrainingStats};

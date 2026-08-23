use core::num::NonZeroU32;

use crate::feature::{FeatureProvider, FeatureSet};
use crate::lattice::{Edge, Lattice};

macro_rules! hashmap {
    ( $($k:expr => $v:expr,)* ) => {
        {
            #[allow(unused_mut)]
            let mut h = HashMap::new();
            $(
                h.insert($k, $v);
            )*
            h
        }
    };
    ( $($k:expr => $v:expr),* ) => {
        hashmap![$( $k => $v, )*]
    };
}

#[cfg(feature = "train")]
macro_rules! logsumexp {
    ( $($x:expr,)* ) => {
        {
            let mut y = f64::NEG_INFINITY;
            $(
                y = $crate::math::logsumexp(y, $x);
            )*
            y
        }
    };
    ( $($x:expr),* ) => {
        logsumexp!($( $x, )*)
    };
}

pub fn generate_test_lattice() -> Lattice {
    let mut lattice = Lattice::new(5).unwrap();
    lattice
        .add_edge(0, Edge::new(1, NonZeroU32::new(1).unwrap()))
        .unwrap();
    lattice
        .add_edge(1, Edge::new(2, NonZeroU32::new(2).unwrap()))
        .unwrap();
    lattice
        .add_edge(2, Edge::new(4, NonZeroU32::new(3).unwrap()))
        .unwrap();
    lattice
        .add_edge(4, Edge::new(5, NonZeroU32::new(4).unwrap()))
        .unwrap();
    lattice
        .add_edge(0, Edge::new(2, NonZeroU32::new(5).unwrap()))
        .unwrap();
    lattice
        .add_edge(2, Edge::new(3, NonZeroU32::new(6).unwrap()))
        .unwrap();
    lattice
        .add_edge(3, Edge::new(4, NonZeroU32::new(7).unwrap()))
        .unwrap();
    lattice
}

/// Deterministically mixes `(index, pos, salt)` into a pseudo-random value.
///
/// A splitmix-style finalizer, so lattice shapes are a pure function of their
/// inputs and never depend on generation order or process state.
fn pseudo(index: usize, pos: usize, salt: usize) -> usize {
    let mut x = (index as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((pos as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((salt as u64).wrapping_mul(0x94D0_49BB_1331_11EB))
        .wrapping_add(0x2545_F491_4F6C_DD1D);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x as usize
}

/// Generates `count` deterministic lattices of varying shape whose labels are
/// in `1..=7`, so `generate_test_feature_provider()` and the weight-index
/// tables of the existing tests cover them.
///
/// Each lattice is 3 to 12 nodes long. Its first edge at every reachable node
/// is a backbone edge (steps of 1 or 2 from node 0 to the end), which is what
/// `calculate_loss` follows as the positive path; skip edges are then added
/// between backbone nodes, so every edge starts and ends on the backbone and
/// no dead ends exist.
pub fn generate_test_lattices(count: usize) -> Vec<Lattice> {
    (0..count)
        .map(|i| {
            let len = 3 + pseudo(i, 0, 0) % 10;
            let mut lattice = Lattice::new(len).unwrap();
            // Backbone: the positive path, added first at every node.
            let mut backbone = vec![0usize];
            let mut pos = 0;
            while pos < len {
                let step = if len - pos >= 2 && pseudo(i, pos, 1) % 2 == 0 {
                    2
                } else {
                    1
                };
                let label = 1 + (pseudo(i, pos, 2) % 7) as u32;
                lattice
                    .add_edge(pos, Edge::new(pos + step, NonZeroU32::new(label).unwrap()))
                    .unwrap();
                pos += step;
                backbone.push(pos);
            }
            // Competing edges between backbone nodes, so the lattice has
            // genuine alternatives without ever leaving the backbone.
            for window in backbone.windows(3) {
                let label = 1 + (pseudo(i, window[0], 3) % 7) as u32;
                lattice
                    .add_edge(
                        window[0],
                        Edge::new(window[2], NonZeroU32::new(label).unwrap()),
                    )
                    .unwrap();
            }
            lattice
        })
        .collect()
}

pub fn generate_test_feature_provider() -> FeatureProvider {
    let mut feature_provider = FeatureProvider::new();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(1).unwrap(), NonZeroU32::new(2).unwrap()],
            &[NonZeroU32::new(1), NonZeroU32::new(2)],
            &[NonZeroU32::new(1), NonZeroU32::new(2)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(3).unwrap(), NonZeroU32::new(4).unwrap()],
            &[NonZeroU32::new(4), NonZeroU32::new(3)],
            &[NonZeroU32::new(3), NonZeroU32::new(4)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(1).unwrap(), NonZeroU32::new(3).unwrap()],
            &[NonZeroU32::new(2), NonZeroU32::new(3)],
            &[NonZeroU32::new(1), NonZeroU32::new(3)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(4).unwrap(), NonZeroU32::new(1).unwrap()],
            &[NonZeroU32::new(2), NonZeroU32::new(1)],
            &[NonZeroU32::new(1), NonZeroU32::new(4)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(2).unwrap(), NonZeroU32::new(3).unwrap()],
            &[NonZeroU32::new(2), NonZeroU32::new(2)],
            &[NonZeroU32::new(2), NonZeroU32::new(3)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(1).unwrap(), NonZeroU32::new(4).unwrap()],
            &[NonZeroU32::new(4), NonZeroU32::new(1)],
            &[NonZeroU32::new(2), NonZeroU32::new(4)],
        ))
        .unwrap();
    feature_provider
        .add_feature_set(FeatureSet::new(
            &[NonZeroU32::new(2).unwrap(), NonZeroU32::new(3).unwrap()],
            &[NonZeroU32::new(3), NonZeroU32::new(4)],
            &[NonZeroU32::new(4), NonZeroU32::new(1)],
        ))
        .unwrap();
    feature_provider
}

pub(crate) use hashmap;

#[cfg(feature = "train")]
pub(crate) use logsumexp;

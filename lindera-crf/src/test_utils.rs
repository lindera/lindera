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

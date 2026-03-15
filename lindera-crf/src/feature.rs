use core::num::NonZeroU32;

use alloc::vec::Vec;

use hashbrown::HashMap;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::errors::{Result, RucrfError};
use crate::utils::FromU32;

#[inline(always)]
pub fn apply_bigram<F>(
    left_label: Option<NonZeroU32>,
    right_label: Option<NonZeroU32>,
    provider: &FeatureProvider,
    bigram_weight_indices: &[HashMap<u32, u32>],
    mut f: F,
) where
    F: FnMut(u32),
{
    match (left_label, right_label) {
        (Some(left_label), Some(right_label)) => {
            if let (Some(left_feature_set), Some(right_feature_set)) = (
                provider.get_feature_set(left_label),
                provider.get_feature_set(right_label),
            ) {
                let left_features = left_feature_set.bigram_left();
                let right_features = right_feature_set.bigram_right();
                for (&left_fid, &right_fid) in left_features.iter().zip(right_features) {
                    if let (Some(left_fid), Some(right_fid)) = (left_fid, right_fid) {
                        let left_fid = usize::from_u32(left_fid.get());
                        let right_fid = right_fid.get();
                        if let Some(&widx) = bigram_weight_indices
                            .get(left_fid)
                            .and_then(|hm| hm.get(&right_fid))
                        {
                            f(widx);
                        }
                    }
                }
            }
        }
        (Some(left_label), None) => {
            if let Some(feature_set) = provider.get_feature_set(left_label) {
                for &left_fid in feature_set.bigram_left() {
                    if let Some(left_fid) = left_fid {
                        let left_fid = usize::from_u32(left_fid.get());
                        if let Some(&widx) = bigram_weight_indices[left_fid].get(&0) {
                            f(widx);
                        }
                    }
                }
            }
        }
        (None, Some(right_label)) => {
            if let Some(feature_set) = provider.get_feature_set(right_label) {
                for &right_fid in feature_set.bigram_right() {
                    if let Some(right_fid) = right_fid {
                        let right_fid = right_fid.get();
                        if let Some(&widx) = bigram_weight_indices[0].get(&right_fid) {
                            f(widx);
                        }
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

/// Manages a set of features for each label.
#[derive(Debug, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct FeatureSet {
    pub(crate) unigram: Vec<NonZeroU32>,
    pub(crate) bigram_right: Vec<Option<NonZeroU32>>,
    pub(crate) bigram_left: Vec<Option<NonZeroU32>>,
}

impl FeatureSet {
    /// Creates a new [`FeatureSet`].
    #[inline(always)]
    #[must_use]
    pub fn new(
        unigram: &[NonZeroU32],
        bigram_right: &[Option<NonZeroU32>],
        bigram_left: &[Option<NonZeroU32>],
    ) -> Self {
        Self {
            unigram: unigram.to_vec(),
            bigram_right: bigram_right.to_vec(),
            bigram_left: bigram_left.to_vec(),
        }
    }

    /// Gets uni-gram feature IDs.
    #[inline(always)]
    #[must_use]
    pub fn unigram(&self) -> &[NonZeroU32] {
        &self.unigram
    }

    /// Gets right bi-gram feature IDs.
    #[inline(always)]
    #[must_use]
    pub fn bigram_right(&self) -> &[Option<NonZeroU32>] {
        &self.bigram_right
    }

    /// Gets left bi-gram feature IDs
    #[inline(always)]
    #[must_use]
    pub fn bigram_left(&self) -> &[Option<NonZeroU32>] {
        &self.bigram_left
    }
}

/// Manages the correspondence between edge labels and feature IDs.
#[derive(Debug, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct FeatureProvider {
    pub(crate) feature_sets: Vec<FeatureSet>,
}

impl FeatureProvider {
    /// Creates a new [`FeatureProvider`].
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the manager has no item.
    #[inline(always)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.feature_sets.is_empty()
    }

    /// Returns the number of items.
    #[inline(always)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.feature_sets.len()
    }

    /// Adds a feature set and returns its ID.
    ///
    /// # Errors
    ///
    /// The number of features must be less than 2^32 - 1.
    #[allow(clippy::missing_panics_doc)]
    #[inline(always)]
    pub fn add_feature_set(&mut self, feature_set: FeatureSet) -> Result<NonZeroU32> {
        let new_id = u32::try_from(self.feature_sets.len() + 1)
            .map_err(|_| RucrfError::model_scale("feature set too large"))?;
        self.feature_sets.push(feature_set);
        Ok(NonZeroU32::new(new_id).unwrap())
    }

    /// Returns the reference to the feature set corresponding to the given ID.
    #[inline(always)]
    pub(crate) fn get_feature_set(&self, label: NonZeroU32) -> Option<&FeatureSet> {
        self.feature_sets
            .get(usize::try_from(label.get() - 1).unwrap())
    }
}

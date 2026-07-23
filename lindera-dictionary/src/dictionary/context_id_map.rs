//! Connection-cost context-ID permutation shipped with a dictionary.
//!
//! When `connection_id_mapping` is enabled, the build relabels left/right context IDs by
//! access frequency so that hot connection-matrix cells cluster in cache. The permutation
//! is applied to the connection matrix, the system dictionary and the unknown dictionary
//! at build time, and is persisted here so that anything compiled later against the same
//! dictionary — most importantly a detailed user dictionary — can be relabeled into the
//! same ID space.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// A pair of context-ID permutations, `perm[old_id] = new_id`.
///
/// `left` permutes left-context IDs (a word's `left_id`, the connection matrix's backward
/// / row axis, length `backward_size`) and `right` permutes right-context IDs (a word's
/// `right_id`, the forward / column axis, length `forward_size`). Both are bijections
/// over `0..len`, with ID 0 pinned to 0 because it is reserved for BOS/EOS.
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct ContextIdMap {
    /// Permutation of left-context IDs (`WordEntry.left_id`, matrix backward axis).
    pub left: Vec<u16>,
    /// Permutation of right-context IDs (`WordEntry.right_id`, matrix forward axis).
    pub right: Vec<u16>,
}

impl ContextIdMap {
    /// Map a left-context ID into the remapped space.
    ///
    /// # Arguments
    ///
    /// * `id` - Left-context ID in the original space.
    ///
    /// # Returns
    ///
    /// The remapped ID, or `id` unchanged when it is outside the permutation (a
    /// malformed ID; the matrix build rejects those separately).
    #[inline]
    pub fn map_left(&self, id: u16) -> u16 {
        self.left.get(id as usize).copied().unwrap_or(id)
    }

    /// Map a right-context ID into the remapped space.
    ///
    /// # Arguments
    ///
    /// * `id` - Right-context ID in the original space.
    ///
    /// # Returns
    ///
    /// The remapped ID, or `id` unchanged when it is outside the permutation.
    #[inline]
    pub fn map_right(&self, id: u16) -> u16 {
        self.right.get(id as usize).copied().unwrap_or(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mapping uses the right axis for each ID kind and passes through out-of-range IDs.
    #[test]
    fn test_map_left_and_right() {
        let map = ContextIdMap {
            left: vec![0, 2, 1],
            right: vec![0, 5, 6, 7],
        };
        assert_eq!(map.map_left(0), 0); // BOS/EOS pinned
        assert_eq!(map.map_left(1), 2);
        assert_eq!(map.map_left(2), 1);
        assert_eq!(map.map_right(3), 7);
        // Out of range on each axis is returned untouched.
        assert_eq!(map.map_left(9), 9);
        assert_eq!(map.map_right(9), 9);
    }
}

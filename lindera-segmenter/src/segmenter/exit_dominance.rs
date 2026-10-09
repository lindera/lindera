//! Exact pruning of the exits that the 1-best segmentation carries across
//! a cut (#1096).
//!
//! An exit of a sentence (a right context id `r` of the words that end it,
//! with the cost `c` of the best path to such a word) continues into the next
//! sentence only through the connection cost from `r` to the left context id
//! `l` of the next word (or of EOS, whose left id is 0). Every other cost of
//! a continuation (the words, the later connections, the penalties) does not
//! depend on the exit. So an exit `e` never beats an exit `f` when
//! `c_e + conn(r_e, l) >= c_f + conn(r_f, l)` for every `l`, i.e. when
//! `c_e - c_f >= margin(r_e, r_f)` with
//! `margin(r_e, r_f) = max over l of (conn(r_f, l) - conn(r_e, l))`.
//!
//! Dropping such an exit leaves the 1-best path unchanged: on equal cost the
//! lattice prefers the BOS edge that comes first, and the BOS edges come in
//! descending order of the exits' edge indices (`LatticeExit::edge_index`,
//! the order in which one lattice over the line prefers them on ties), so
//! `e` is dropped on equality only when `f`'s edge index is the greater one.
//! After `、` and `。`, the exits are a few words that end with
//! the mark (the punctuation entry and unknown symbol words), and one of
//! them almost always dominates the others, so a single state is carried and
//! the context costs nearly nothing.
//!
//! When a later sentence has no path, the segment ends early and the states
//! are compared with the EOS connection, whose left id is one of the `l`
//! above, so a dropped exit cannot win there either (#1133).
//!
//! The N-best search keeps every exit: a dominated exit still has paths
//! among the `n` best.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::viterbi::LatticeExit;

/// Number of slots of the margin cache; a power of two.
const MARGIN_SLOTS: usize = 4096;

/// A cache of [`margin`]s per pair of right context ids, shared by the
/// clones of a segmenter and safe to use from several threads.
///
/// Direct-mapped: each slot holds one pair's margin, packed with the pair as
/// its tag; a pair that maps to an occupied slot replaces it. Concurrent
/// writers store the same value for the same pair, so relaxed atomics
/// suffice.
#[derive(Debug, Default)]
pub(super) struct MarginCache {
    /// The slots, allocated on first use: `(tag << 32) | margin as u32`,
    /// where the tag is the packed pair plus one, so 0 is an empty slot.
    slots: OnceLock<Box<[AtomicU64]>>,
}

impl MarginCache {
    /// Returns the margin of an exit with right id `from` over an exit with
    /// right id `over`, from the cache or computed and cached.
    ///
    /// # 引数
    ///
    /// * `matrix` - The connection cost matrix of the segmenter's
    ///   dictionary.
    /// * `from` - The right context id of the exit that may be dominated.
    /// * `over` - The right context id of the exit that may dominate it;
    ///   not `from`.
    ///
    /// # 戻り値
    ///
    /// See [`margin`].
    pub(super) fn margin(&self, matrix: &ConnectionCostMatrix, from: u16, over: u16) -> i32 {
        let slots = self.slots.get_or_init(|| {
            (0..MARGIN_SLOTS)
                .map(|_| AtomicU64::new(0))
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });
        let pair = (u32::from(from) << 16) | u32::from(over);
        // `from != over`, so the pair is never `u32::MAX` and the tag is
        // never 0.
        let tag = pair.wrapping_add(1);
        let index =
            (pair.wrapping_mul(0x9E37_79B1) >> (32 - MARGIN_SLOTS.trailing_zeros())) as usize;
        let slot = &slots[index];
        let packed = slot.load(Ordering::Relaxed);
        if (packed >> 32) as u32 == tag {
            return packed as u32 as i32;
        }
        let value = margin(matrix, from, over);
        slot.store(
            (u64::from(tag) << 32) | u64::from(value as u32),
            Ordering::Relaxed,
        );
        value
    }
}

/// Returns how much more, at most, a continuation costs from right context
/// id `from` than from `over`: the maximum over every left context id `l` of
/// `conn(over, l) - conn(from, l)`.
///
/// # 引数
///
/// * `matrix` - The connection cost matrix.
/// * `from` - The right context id of the exit that may be dominated.
/// * `over` - The right context id of the exit that may dominate it.
///
/// # 戻り値
///
/// The margin; `i32::MAX` (nothing is dominated) for a matrix without left
/// ids or a right id outside the matrix.
fn margin(matrix: &ConnectionCostMatrix, from: u16, over: u16) -> i32 {
    let forward_size = matrix.forward_size as usize;
    let (from, over) = (usize::from(from), usize::from(over));
    if from >= forward_size || over >= forward_size || matrix.backward_size == 0 {
        return i32::MAX;
    }
    // The matrix stores one row per left id, indexed by right id.
    matrix
        .costs()
        .chunks_exact(forward_size)
        .map(|row| i32::from(row[over]) - i32::from(row[from]))
        .max()
        .unwrap_or(i32::MAX)
}

/// Returns whether exit `f` dominates exit `e`: `e` never gives the best
/// path through the next word, whatever its left context id, so dropping
/// `e` leaves the 1-best path unchanged.
///
/// # Arguments
///
/// * `f` - The edge index (`LatticeExit::edge_index`) and the cost of the
///   dominating exit.
/// * `e` - The edge index and the cost of the dominated exit, an exit with
///   a right context id other than `f`'s.
/// * `margin` - `margin(r_e, r_f)` (see [`margin`]); `i32::MAX` dominates
///   nothing.
///
/// # Returns
///
/// `true` when `c_e - c_f` exceeds the margin, or equals it and `f`'s BOS
/// edge comes first (the greater edge index), which wins the ties.
fn dominates(f: (u32, i32), e: (u32, i32), margin: i32) -> bool {
    if margin == i32::MAX {
        return false;
    }
    let gap = i64::from(e.1) - i64::from(f.1);
    let margin = i64::from(margin);
    gap > margin || (gap == margin && f.0 > e.0)
}

/// Drops the exits that another exit dominates (see the module
/// documentation), keeping the order of the others.
///
/// # 引数
///
/// * `exits` - The exits of a sentence, with distinct right ids.
/// * `cache` - The margin cache of the segmenter.
/// * `matrix` - The connection cost matrix of the segmenter's dictionary.
/// * `dominated` - Scratch buffer.
pub(super) fn prune_dominated(
    exits: &mut Vec<LatticeExit>,
    cache: &MarginCache,
    matrix: &ConnectionCostMatrix,
    dominated: &mut Vec<bool>,
) {
    if exits.len() < 2 {
        return;
    }
    dominated.clear();
    dominated.extend(exits.iter().map(|exit| {
        exits.iter().any(|other| {
            other.right_id() != exit.right_id()
                && dominates(
                    (other.edge_index(), other.cost()),
                    (exit.edge_index(), exit.cost()),
                    cache.margin(matrix, exit.right_id(), other.right_id()),
                )
        })
    }));
    // Dominance is transitive and has no cycle (the tie rule follows the
    // edge indices, which are distinct), so the cheapest exit of every chain
    // survives.
    let mut index = 0;
    exits.retain(|_| {
        let keep = !dominated[index];
        index += 1;
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::dominates;

    /// On a gap equal to the margin, the exit whose BOS edge comes first
    /// (the greater edge index, which one lattice over the line keeps on
    /// ties) wins the ties, so only the other one is dominated (#1140).
    #[test]
    fn test_dominates_breaks_ties_by_edge_index() {
        // Above the margin, either way round.
        assert!(dominates((3, 100), (7, 151), 50));
        assert!(dominates((7, 100), (3, 151), 50));
        // At the margin: only the exit with the lower edge index loses.
        assert!(dominates((7, 100), (3, 150), 50));
        assert!(!dominates((3, 100), (7, 150), 50));
        // Below the margin, nothing is dominated.
        assert!(!dominates((3, 100), (7, 149), 50));
        // Negative margins and extreme costs.
        assert!(dominates((3, 0), (7, -10), -11));
        assert!(!dominates((3, i32::MAX), (7, i32::MIN), 0));
        assert!(dominates((3, i32::MIN), (7, i32::MAX), i32::MAX - 1));
        // The margin of a right id outside the matrix dominates nothing.
        assert!(!dominates((3, 0), (7, i32::MAX), i32::MAX));
        assert!(!dominates((3, i32::MIN), (7, i32::MAX), i32::MAX));
    }

    #[cfg(feature = "embed-ipadic")]
    #[test]
    fn test_margin_matches_the_matrix() {
        use super::{MarginCache, margin};

        let dictionary = crate::dictionary::load_dictionary("embedded://ipadic").unwrap();
        let matrix = &dictionary.connection_cost_matrix;
        let cache = MarginCache::default();
        for (from, over) in [(0_u16, 1_u16), (1, 0), (5, 9), (9, 5), (1000, 3)] {
            let expected = (0..matrix.backward_size)
                .map(|left| matrix.cost(u32::from(over), left) - matrix.cost(u32::from(from), left))
                .max()
                .unwrap();
            assert_eq!(margin(matrix, from, over), expected);
            // Twice: computed, then cached.
            assert_eq!(cache.margin(matrix, from, over), expected);
            assert_eq!(cache.margin(matrix, from, over), expected);
        }
        assert_eq!(margin(matrix, u16::MAX, 0), i32::MAX);
    }
}

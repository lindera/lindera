//! The bigram weight indices of the training lattices, looked up once.

use core::num::NonZeroU32;

use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::feature::{self, FeatureProvider};
use crate::lattice::Lattice;
use crate::utils::FromU32;

/// The context id of BOS on the left of a pair, and of EOS on the right.
const SENTENCE_BOUNDARY: u32 = 0;

/// Converts a length or an id to `u32`.
///
/// # Arguments
///
/// * `value` - The value.
///
/// # Returns
///
/// The value as `u32`.
///
/// # Panics
///
/// Panics when the value does not fit, which would take more than 2^32
/// junctions, pairs or weight indices.
fn to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("the bigram table has more than 2^32 entries")
}

/// The pair ids of one lattice: one for every junction of an incoming and
/// an outgoing edge of a node, and one for every step of the gold path.
pub struct LatticePairs {
    /// Where the junctions of each node start in `junctions`, followed by
    /// the end of the last node's.
    node_offsets: Vec<u32>,
    /// The pair id of every junction. The junctions of a node form a matrix
    /// in row-major order: one row per entry the node has in the forward
    /// scores (BOS at node 0, then the incoming edges in the order
    /// `forward_backward::calculate_alphas_betas` adds them), one column per
    /// entry in the backward scores (EOS at the last node, then the outgoing
    /// edges in their order).
    junctions: Vec<u32>,
    /// The pair ids of the gold path, from the pair out of BOS to the pair
    /// into EOS.
    gold: Vec<u32>,
}

impl LatticePairs {
    /// Returns the pair ids of the junctions of `node`, row-major.
    ///
    /// # Arguments
    ///
    /// * `node` - The node index.
    ///
    /// # Returns
    ///
    /// The node's junction matrix.
    #[inline(always)]
    pub fn junctions(&self, node: usize) -> &[u32] {
        let start = usize::from_u32(self.node_offsets[node]);
        let end = usize::from_u32(self.node_offsets[node + 1]);
        &self.junctions[start..end]
    }

    /// Returns the pair ids of the gold path, the last one being the pair
    /// into EOS.
    ///
    /// # Returns
    ///
    /// The pair ids, one per gold edge plus one.
    #[inline(always)]
    pub fn gold(&self) -> &[u32] {
        &self.gold
    }
}

/// The bigram weight indices of every pair of edges that meet in a set of
/// training lattices.
///
/// [`feature::apply_bigram`] finds the weight indices of a pair of labels
/// with one map lookup per bigram template. During training the lattices,
/// the feature sets and the weight-index maps do not change, so the table
/// looks the indices of every pair up once, in the order `apply_bigram`
/// yields them, and an evaluation adds the weights in that same order.
///
/// `apply_bigram` reads only the left label's `bigram_left` ids and the
/// right label's `bigram_right` ids (or the absence of a feature set), so
/// labels that share them share a context id, and the indices are stored
/// once per pair of contexts.
pub struct BigramTable {
    /// Where each pair's weight indices start in `indices`, followed by the
    /// end of the last pair's.
    offsets: Vec<u32>,
    /// The weight indices of every pair, in the order `apply_bigram` yields
    /// them.
    indices: Vec<u32>,
    /// The pair ids of each lattice.
    lattices: Vec<LatticePairs>,
}

impl BigramTable {
    /// Builds the table for the given lattices.
    ///
    /// # Arguments
    ///
    /// * `lattices` - The training lattices.
    /// * `provider` - The feature sets of the labels.
    /// * `bigram_weight_indices` - The weight index of each bigram feature
    ///   pair.
    ///
    /// # Returns
    ///
    /// The table, with one [`LatticePairs`] per lattice, in slice order.
    pub fn new(
        lattices: &[Lattice],
        provider: &FeatureProvider,
        bigram_weight_indices: &[HashMap<u32, u32>],
    ) -> Self {
        let mut builder = Builder::new(provider, bigram_weight_indices);
        let lattices = lattices
            .iter()
            .map(|lattice| builder.lattice_pairs(lattice))
            .collect();
        Self {
            offsets: builder.offsets,
            indices: builder.indices,
            lattices,
        }
    }

    /// Returns the weight indices of a pair, in the order
    /// [`feature::apply_bigram`] yields them.
    ///
    /// # Arguments
    ///
    /// * `pair` - The pair id.
    ///
    /// # Returns
    ///
    /// The weight indices.
    #[inline(always)]
    pub fn indices(&self, pair: u32) -> &[u32] {
        let pair = usize::from_u32(pair);
        let start = usize::from_u32(self.offsets[pair]);
        let end = usize::from_u32(self.offsets[pair + 1]);
        &self.indices[start..end]
    }

    /// Returns the pair ids of a lattice.
    ///
    /// # Arguments
    ///
    /// * `index` - The lattice's index in the slice the table was built from.
    ///
    /// # Returns
    ///
    /// The lattice's pair ids.
    #[inline(always)]
    pub fn lattice(&self, index: usize) -> &LatticePairs {
        &self.lattices[index]
    }

    /// Returns the number of distinct pairs.
    #[cfg(test)]
    pub fn pair_count(&self) -> usize {
        self.offsets.len() - 1
    }
}

/// The bigram ids `apply_bigram` reads from one side of a label: the ids
/// of its feature set, or `None` when the label has no feature set.
type Context<'a> = Option<&'a [Option<NonZeroU32>]>;

/// Builds a [`BigramTable`], interning contexts and pairs as it goes.
struct Builder<'a> {
    provider: &'a FeatureProvider,
    bigram_weight_indices: &'a [HashMap<u32, u32>],
    /// The context id of each left context.
    left_ids: HashMap<Context<'a>, u32>,
    /// The context id of each right context.
    right_ids: HashMap<Context<'a>, u32>,
    /// A label with each left context id; `None` for BOS.
    left_labels: Vec<Option<NonZeroU32>>,
    /// A label with each right context id; `None` for EOS.
    right_labels: Vec<Option<NonZeroU32>>,
    /// The (left context id, right context id) of each label seen so far.
    label_contexts: HashMap<NonZeroU32, (u32, u32)>,
    /// The pair id of each (left context id, right context id).
    pair_ids: HashMap<(u32, u32), u32>,
    offsets: Vec<u32>,
    indices: Vec<u32>,
}

impl<'a> Builder<'a> {
    fn new(provider: &'a FeatureProvider, bigram_weight_indices: &'a [HashMap<u32, u32>]) -> Self {
        Self {
            provider,
            bigram_weight_indices,
            left_ids: HashMap::new(),
            right_ids: HashMap::new(),
            left_labels: vec![None],
            right_labels: vec![None],
            label_contexts: HashMap::new(),
            pair_ids: HashMap::new(),
            offsets: vec![0],
            indices: vec![],
        }
    }

    /// Returns the (left, right) context ids of a label.
    fn contexts(&mut self, label: NonZeroU32) -> (u32, u32) {
        if let Some(&ids) = self.label_contexts.get(&label) {
            return ids;
        }
        let feature_set = self.provider.get_feature_set(label);
        let left = feature_set.map(|feature_set| feature_set.bigram_left());
        let right = feature_set.map(|feature_set| feature_set.bigram_right());
        let left_id = *self.left_ids.entry(left).or_insert_with(|| {
            self.left_labels.push(Some(label));
            to_u32(self.left_labels.len() - 1)
        });
        let right_id = *self.right_ids.entry(right).or_insert_with(|| {
            self.right_labels.push(Some(label));
            to_u32(self.right_labels.len() - 1)
        });
        self.label_contexts.insert(label, (left_id, right_id));
        (left_id, right_id)
    }

    /// Returns the pair id of a (left context id, right context id),
    /// collecting its weight indices the first time.
    fn pair(&mut self, left: u32, right: u32) -> u32 {
        if let Some(&id) = self.pair_ids.get(&(left, right)) {
            return id;
        }
        let indices = &mut self.indices;
        feature::apply_bigram(
            self.left_labels[usize::from_u32(left)],
            self.right_labels[usize::from_u32(right)],
            self.provider,
            self.bigram_weight_indices,
            |widx| indices.push(widx),
        );
        self.offsets.push(to_u32(self.indices.len()));
        let id = to_u32(self.pair_ids.len());
        self.pair_ids.insert((left, right), id);
        id
    }

    /// Returns the pair ids of one lattice.
    fn lattice_pairs(&mut self, lattice: &Lattice) -> LatticePairs {
        let nodes = lattice.nodes();
        let len = nodes.len();

        // The left context of every row and the right context of every
        // column of each node's junctions, in the order the forward and the
        // backward scores list them.
        let mut rows: Vec<Vec<u32>> = vec![vec![]; len];
        let mut columns: Vec<Vec<u32>> = vec![vec![]; len];
        rows[0].push(SENTENCE_BOUNDARY);
        columns[len - 1].push(SENTENCE_BOUNDARY);
        for (i, node) in nodes.iter().enumerate() {
            for edge in node.edges() {
                let (left, right) = self.contexts(edge.label());
                rows[edge.target()].push(left);
                columns[i].push(right);
            }
        }

        let mut node_offsets = Vec::with_capacity(len + 1);
        let mut junctions = vec![];
        for (row, column) in rows.iter().zip(&columns) {
            node_offsets.push(to_u32(junctions.len()));
            for &left in row {
                for &right in column {
                    junctions.push(self.pair(left, right));
                }
            }
        }
        node_offsets.push(to_u32(junctions.len()));

        let mut gold = vec![];
        let mut pos = 0;
        let mut prev = SENTENCE_BOUNDARY;
        while pos < len - 1 {
            let edge = &nodes[pos].edges()[0];
            let (left, right) = self.contexts(edge.label());
            gold.push(self.pair(prev, right));
            prev = left;
            pos = edge.target();
        }
        gold.push(self.pair(prev, SENTENCE_BOUNDARY));

        LatticePairs {
            node_offsets,
            junctions,
            gold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::{self, hashmap};

    /// Bigram weight indices with gaps, so that `apply_bigram` skips some
    /// template slots.
    fn sparse_bigram_weight_indices() -> Vec<HashMap<u32, u32>> {
        vec![
            hashmap![1 => 0, 3 => 1, 4 => 2],
            hashmap![0 => 3, 1 => 4, 2 => 5],
            hashmap![0 => 6, 2 => 7, 3 => 8, 4 => 9],
            hashmap![1 => 10, 4 => 11],
            hashmap![0 => 12, 1 => 13, 2 => 14, 3 => 15],
        ]
    }

    /// Returns what `apply_bigram` yields for a pair of labels.
    fn yielded(
        left: Option<NonZeroU32>,
        right: Option<NonZeroU32>,
        provider: &FeatureProvider,
        bigram_weight_indices: &[HashMap<u32, u32>],
    ) -> Vec<u32> {
        let mut indices = vec![];
        feature::apply_bigram(left, right, provider, bigram_weight_indices, |widx| {
            indices.push(widx)
        });
        indices
    }

    #[test]
    fn indices_are_what_apply_bigram_yields() {
        let provider = test_utils::generate_irregular_feature_provider();
        let bigram_weight_indices = sparse_bigram_weight_indices();
        let mut lattices = test_utils::generate_irregular_lattices(18);
        lattices.extend(test_utils::generate_test_lattices(30));
        let table = BigramTable::new(&lattices, &provider, &bigram_weight_indices);

        let mut junction_count = 0;
        let mut empty_pairs = 0;
        for (index, lattice) in lattices.iter().enumerate() {
            let pairs = table.lattice(index);
            let nodes = lattice.nodes();

            // The labels of each node's rows and columns, in the order the
            // forward and the backward scores list them.
            let mut rows = vec![vec![]; nodes.len()];
            let mut columns = vec![vec![]; nodes.len()];
            rows[0].push(None);
            columns[nodes.len() - 1].push(None);
            for (i, node) in nodes.iter().enumerate() {
                for edge in node.edges() {
                    rows[edge.target()].push(Some(edge.label()));
                    columns[i].push(Some(edge.label()));
                }
            }
            for (node, (row_labels, column_labels)) in rows.iter().zip(&columns).enumerate() {
                let junctions = pairs.junctions(node);
                assert_eq!(
                    junctions.len(),
                    row_labels.len() * column_labels.len(),
                    "lattice {index}, node {node}: junction count"
                );
                for (row, &left) in row_labels.iter().enumerate() {
                    for (column, &right) in column_labels.iter().enumerate() {
                        let pair = junctions[row * column_labels.len() + column];
                        let expected = yielded(left, right, &provider, &bigram_weight_indices);
                        empty_pairs += usize::from(expected.is_empty());
                        assert_eq!(
                            table.indices(pair),
                            expected.as_slice(),
                            "lattice {index}, node {node}, row {row}, column {column}"
                        );
                    }
                }
                junction_count += junctions.len();
            }

            let mut gold = vec![];
            let mut pos = 0;
            let mut prev = None;
            while pos < nodes.len() - 1 {
                let edge = nodes[pos].edges()[0];
                gold.push(yielded(
                    prev,
                    Some(edge.label()),
                    &provider,
                    &bigram_weight_indices,
                ));
                prev = Some(edge.label());
                pos = edge.target();
            }
            gold.push(yielded(prev, None, &provider, &bigram_weight_indices));
            let got: Vec<Vec<u32>> = pairs
                .gold()
                .iter()
                .map(|&pair| table.indices(pair).to_vec())
                .collect();
            assert_eq!(got, gold, "lattice {index}: gold path");
        }

        // The fixture reaches the pairs for which apply_bigram yields
        // nothing (no feature set, empty slots, missing weight indices)...
        assert!(empty_pairs > 0, "no junction without weight indices");
        // ...and labels that share their bigram ids share pairs.
        assert!(
            table.pair_count() < junction_count,
            "{} pairs for {junction_count} junctions",
            table.pair_count()
        );
    }
}

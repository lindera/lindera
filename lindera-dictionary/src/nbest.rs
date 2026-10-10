use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::viterbi::{Lattice, LatticeExit, NBestPath, TokenOffset};

/// An element in the A* priority queue for N-Best search.
/// Represents a partial path from EOS (or from a final edge, see
/// [`NBestGenerator::from_exit`]) backward toward BOS.
#[derive(Clone, Debug)]
struct QueueElement {
    /// The `ends_at` slot holding the current edge (where it ends, unless it
    /// was carried over skipped whitespace)
    char_pos: u32,
    /// Index of the current edge in `ends_at[char_pos]`
    edge_index: u32,
    /// f(x) = g(x) + h(x) -- total estimated cost
    fx: i64,
    /// g(x) = accumulated real cost from EOS backward to this point
    gx: i64,
    /// Link to the previous QueueElement in the elements chain (toward EOS)
    prev: Option<usize>,
    /// Push order (see [`NBestGenerator::push`]): of two elements with the
    /// same `fx`, the one pushed later is popped first.
    seq: u64,
}

/// Min-heap ordering: lower fx = higher priority; on equal fx, the element
/// pushed later (higher `seq`) has the higher priority, so equal-cost
/// partial paths are explored depth first, the last pushed first (#1131).
impl Ord for QueueElement {
    /// Orders by `fx` reversed, then by `seq`: the greatest element, the
    /// one [`BinaryHeap`] pops first, has the lowest `fx` and, among those,
    /// the highest `seq`.
    ///
    /// # Arguments
    ///
    /// * `other` - The element to compare with.
    ///
    /// # Returns
    ///
    /// The ordering of `self` relative to `other`.
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed for min-heap.
        other.fx.cmp(&self.fx).then(self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for QueueElement {
    /// The total order of `cmp`.
    ///
    /// # Arguments
    ///
    /// * `other` - The element to compare with.
    ///
    /// # Returns
    ///
    /// Always `Some`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for QueueElement {}

impl PartialEq for QueueElement {
    /// Compares the fields `cmp` orders by, `fx` and `seq`, so equality
    /// agrees with the ordering.
    ///
    /// # Arguments
    ///
    /// * `other` - The element to compare with.
    ///
    /// # Returns
    ///
    /// Whether both have the same `fx` and `seq`.
    fn eq(&self, other: &Self) -> bool {
        self.fx == other.fx && self.seq == other.seq
    }
}

/// Generates N-best paths through a Lattice using Backward A* search.
///
/// After forward Viterbi (set_text_nbest), this generator uses the recorded
/// all_paths transitions and path_cost heuristics to enumerate paths
/// from EOS to BOS in order of increasing total cost.
///
/// The heuristic of an edge is its forward path cost, the exact cost of the
/// best path from any BOS edge to it (the BOS edge's own cost included), so
/// the paths come out in ascending order of their total cost, BOS cost
/// included, also with several BOS edges
/// ([`LatticeOptions::bos`](crate::viterbi::LatticeOptions::bos)).
///
/// The first path is the lattice's best path, the one
/// [`Lattice::tokens_offset`] backtraces (from an exit, the one
/// [`Lattice::exit_tokens_offset_into`] backtraces), with its BOS index,
/// even when other paths cost the same: the search follows the left edge
/// the forward pass keeps, the last of equal-cost ones in its slot, before
/// the others (#1131, #1135).
pub struct NBestGenerator<'a> {
    /// The N-best lattice the paths are taken from.
    lattice: &'a Lattice,
    /// The A* frontier: partial paths ordered by `fx`, lowest first.
    queue: BinaryHeap<QueueElement>,
    /// Storage for QueueElement chain (for path reconstruction)
    elements: Vec<QueueElement>,
    /// The number of elements pushed so far, the next element's `seq`.
    pushed: u64,
}

impl<'a> NBestGenerator<'a> {
    /// Initialize the generator from a lattice that has been processed
    /// with set_text_nbest(): the paths through EOS, with the EOS
    /// connection in their costs.
    ///
    /// # 引数
    ///
    /// * `lattice` - The N-best lattice.
    ///
    /// # 戻り値
    ///
    /// The generator; it yields nothing when the lattice holds no complete
    /// path.
    pub fn new(lattice: &'a Lattice) -> Self {
        let mut generator = NBestGenerator {
            lattice,
            queue: BinaryHeap::new(),
            elements: Vec::new(),
            pushed: 0,
        };
        generator.init();
        generator
    }

    /// Initializes the generator for the paths that end with the right
    /// context id of `exit`, without EOS: every path from a BOS edge to an
    /// edge that ends at the end of the sentence and has that right id, in
    /// ascending order of the cost [`LatticeExit::cost`] measures (the last
    /// word's Decompose length penalty included, the EOS connection not).
    /// The first path is the exit's own path
    /// ([`Lattice::exit_tokens_offset_into`]) and costs `exit.cost()`.
    ///
    /// The search starts from all those final edges, each with its exit
    /// penalty as the cost accumulated so far. For a sentence without words
    /// (one of skipped whitespace, or an empty one) the final edges are the
    /// BOS edges themselves, so it yields one empty path per BOS edge with
    /// that right id, at that edge's cost.
    ///
    /// # 引数
    ///
    /// * `lattice` - The N-best lattice (`set_text_nbest*`).
    /// * `exit` - An exit of the lattice's current sentence, from
    ///   [`Lattice::exits_into`]; only its right id is read.
    ///
    /// # 戻り値
    ///
    /// The generator; it yields nothing when no final edge has the exit's
    /// right id.
    pub fn from_exit(lattice: &'a Lattice, exit: &LatticeExit) -> Self {
        let mut generator = NBestGenerator {
            lattice,
            queue: BinaryHeap::new(),
            elements: Vec::new(),
            pushed: 0,
        };
        let char_pos = lattice.char_len() as u32;
        // In slot order: of the final edges with the same cost, the last one
        // is popped first, as the exit keeps it (see `push`).
        for (i, edge) in lattice.final_edges().iter().enumerate() {
            if edge.right_id() != exit.right_id() {
                continue;
            }
            let gx = lattice.exit_penalty(i) as i64;
            generator.push(char_pos, i as u32, edge.path_cost() as i64 + gx, gx, None);
        }
        generator
    }

    /// Seeds the queue with the EOS edge of the lattice, if any.
    fn init(&mut self) {
        let char_len = self.lattice.char_len();
        let eos_edges = self.lattice.edges_at_char(char_len);
        if eos_edges.is_empty() {
            return;
        }

        // EOS is the last edge pushed to ends_at[char_len]
        let eos_index = (eos_edges.len() - 1) as u32;
        let eos_edge = &eos_edges[eos_index as usize];

        // Initial element: start from EOS with g(x)=0
        self.push(
            char_len as u32,
            eos_index,
            eos_edge.path_cost() as i64,
            0,
            None,
        );
    }

    /// Pushes a queue element, numbering it in push order.
    ///
    /// Of the elements with the same `fx`, the last pushed is popped first.
    /// The callers push the predecessors of an edge in slot order (its
    /// `PathEntry` run lists them in that order, the order the forward
    /// relaxation scanned them in), and [`NBestGenerator::from_exit`]
    /// pushes the final edges in slot order too. So among equal-cost
    /// predecessors the search takes the last one, the one the forward
    /// relaxation keeps (it keeps the last of equal-cost left edges, #1135),
    /// and follows it depth first: the first path found is the path the
    /// lattice backtraces, even when other paths cost the same (#1131).
    ///
    /// # Arguments
    ///
    /// * `char_pos` - The slot of the element's edge.
    /// * `edge_index` - The index of the edge in its slot.
    /// * `fx` - The estimated total cost.
    /// * `gx` - The cost accumulated from EOS.
    /// * `prev` - The element it was expanded from.
    fn push(&mut self, char_pos: u32, edge_index: u32, fx: i64, gx: i64, prev: Option<usize>) {
        let seq = self.pushed;
        self.pushed += 1;
        self.queue.push(QueueElement {
            char_pos,
            edge_index,
            fx,
            gx,
            prev,
            seq,
        });
    }

    /// Returns the next best path as (path, cost).
    /// The path is a vector of (byte_start, byte_end, WordId) triples, with
    /// the token ends of `Lattice::tokens_offset`: the whitespace an entry
    /// ends with is part of its token, skipped whitespace is not.
    /// The cost is the total path cost (fx at BOS), lower is better.
    /// Returns None when no more paths are available.
    ///
    /// # 戻り値
    ///
    /// The next path and its cost, or `None` when there are no more; see
    /// [`NBestGenerator::next_with_bos`], which also returns the BOS index.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<NBestPath> {
        self.next_with_bos().map(|(path, _)| path)
    }

    /// Returns the next best path as (path, cost), like
    /// [`NBestGenerator::next`], with the BOS index of the path.
    ///
    /// # 戻り値
    ///
    /// The next path and its cost, lower is better (the BOS edge's cost
    /// included), and its BOS index: the index in
    /// [`LatticeOptions::bos`](crate::viterbi::LatticeOptions::bos) of the
    /// context the path starts from, always 0 for the default single BOS
    /// edge. `None` when there are no more paths.
    pub fn next_with_bos(&mut self) -> Option<(NBestPath, usize)> {
        while let Some(current) = self.queue.pop() {
            let char_pos = current.char_pos as usize;
            let edge_index = current.edge_index as usize;

            let edges = self.lattice.edges_at_char(char_pos);
            if edge_index >= edges.len() {
                continue;
            }
            let edge = &edges[edge_index];

            // Check if we reached BOS (left_index == u32::MAX means no
            // predecessor = BOS). The BOS edges are the first edges of the
            // slot holding them, in reverse context order, which `bos_index`
            // maps back.
            if edge.left_index() == u32::MAX {
                return Some((
                    (self.reconstruct_path(&current), current.fx),
                    self.lattice.bos_index(edge_index),
                ));
            }

            // Store current element for chain linking
            let current_idx = self.elements.len();
            self.elements.push(current.clone());

            // Expand: for each predecessor path of this edge.
            //
            // `paths_at_char(char_pos)` is always partitioned into contiguous,
            // strictly-ascending-by-`edge_index` runs: every push site
            // (`push_relaxed_nbest`, which records the transitions of a
            // dictionary word or of one length of an unknown word, and
            // `set_text_nbest`'s EOS-connect block, all in viterbi.rs) writes
            // every `PathEntry` for one edge in one go, using the target
            // slot's length at that time as that edge's index, before any
            // other edge targeting the same stop position can push into this
            // same `all_paths` vector. So the target edge's entries form one
            // contiguous run, locatable via binary search instead of a full
            // linear scan.
            let paths = self.lattice.paths_at_char(char_pos);
            let start = paths.partition_point(|p| p.edge_index() < edge_index as u32);
            let end = paths.partition_point(|p| p.edge_index() <= edge_index as u32);
            // In slot order: see `push`.
            for path_entry in &paths[start..end] {
                let left_pos = path_entry.left_pos() as usize;
                let left_index = path_entry.left_index() as usize;

                let left_edges = self.lattice.edges_at_char(left_pos);
                if left_index >= left_edges.len() {
                    continue;
                }
                let left_edge = &left_edges[left_index];

                // g(x) for the predecessor:
                // path_entry.cost = left_edge.path_cost + conn_cost + penalty
                // conn_and_penalty = path_entry.cost - left_edge.path_cost
                // new_gx = current.gx + conn_and_penalty + edge.word_cost
                let conn_and_penalty = path_entry.cost() as i64 - left_edge.path_cost() as i64;
                let new_gx = current.gx + conn_and_penalty + edge.word_cost() as i64;

                // f(x) = h(x) + g(x), where h(x) = left_edge.path_cost
                let new_fx = left_edge.path_cost() as i64 + new_gx;

                self.push(
                    left_pos as u32,
                    left_index as u32,
                    new_fx,
                    new_gx,
                    Some(current_idx),
                );
            }
        }
        None
    }

    /// Rebuilds the tokens of the path that reached BOS at `bos_elem`, from
    /// its chain of elements: every edge on it but EOS. A path started by
    /// [`NBestGenerator::from_exit`] has no EOS, and its first element, a
    /// final word edge, is a token like any other.
    ///
    /// # 引数
    ///
    /// * `bos_elem` - The element of the BOS edge the path reached.
    ///
    /// # 戻り値
    ///
    /// The tokens in reading order; empty for a path without words.
    fn reconstruct_path(&self, bos_elem: &QueueElement) -> Vec<TokenOffset> {
        let mut path = Vec::new();
        let mut maybe_idx = bos_elem.prev;

        // Walk the chain from BOS toward EOS via prev links.
        // The chain visits edges in forward order (first word first)
        // because each element's prev points toward EOS.
        while let Some(idx) = maybe_idx {
            let elem = &self.elements[idx];
            let edges = self.lattice.edges_at_char(elem.char_pos as usize);
            let edge = &edges[elem.edge_index as usize];

            // Skip the EOS edge: it is the only edge whose start equals the
            // slot it is stored in (a zero-length span at char_len; a
            // carried edge always starts before its slot).
            if edge.start_char() as u32 != elem.char_pos {
                path.push((
                    self.lattice.byte_offset_of(edge.start_char() as usize),
                    self.lattice.edge_end_byte(edge, elem.char_pos as usize),
                    edge.word_id(),
                ));
            }

            maybe_idx = elem.prev;
        }

        path
    }
}

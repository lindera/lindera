use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::viterbi::{Edge, Lattice, LatticeExit, NBestPath, PathEntry, TokenOffset};

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
///
/// Every path comes out, also the many that share their word boundaries;
/// for one path per segmentation (unique N-best) use
/// [`UniqueNBestGenerator`], which does not enumerate them.
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

    /// Seeds the queue with the EOS edge of the lattice, if any: a lattice
    /// whose EOS did not connect has no complete path, even when words end
    /// at the end of the sentence.
    fn init(&mut self) {
        let char_len = self.lattice.char_len();
        let Some(eos_index) = self.lattice.eos_index() else {
            return;
        };
        let eos_edge = &self.lattice.edges_at_char(char_len)[eos_index];

        // Initial element: start from EOS with g(x)=0
        self.push(
            char_len as u32,
            eos_index as u32,
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

            // Expand: for each predecessor path of this edge, in slot order
            // (see `push`).
            for path_entry in transitions(self.lattice, char_pos, edge_index as u32) {
                // Every transition of an edge comes from the slot where the
                // edge starts: only stored edges record transitions.
                debug_assert_eq!(path_entry.left_pos(), u32::from(edge.start_char()));
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
            let slot = elem.char_pos as usize;
            let edge = &self.lattice.edges_at_char(slot)[elem.edge_index as usize];
            if let Some(token) = token(self.lattice, slot, edge) {
                path.push(token);
            }
            maybe_idx = elem.prev;
        }

        path
    }
}

/// Returns the transitions into one edge: its run of `PathEntry`s in
/// `paths_at_char(slot)`, in the order of their left edges in the left slot.
///
/// `paths_at_char(slot)` is always partitioned into contiguous,
/// strictly-ascending-by-`edge_index` runs: every push site
/// (`push_relaxed_nbest`, which records the transitions of a dictionary word
/// or of one length of an unknown word, and `set_text_nbest`'s EOS-connect
/// block, all in viterbi.rs) writes every `PathEntry` for one stored edge in
/// one go, using the target slot's length at that time as that edge's index,
/// before any other edge targeting the same stop position can push into this
/// same `all_paths` vector. So the target edge's entries form one contiguous
/// run, located by binary search instead of a full linear scan. Every entry
/// of the run comes from the slot where the edge starts.
///
/// # Arguments
///
/// * `lattice` - The N-best lattice.
/// * `slot` - The slot holding the edge.
/// * `edge_index` - The index of the edge in its slot.
///
/// # Returns
///
/// The edge's transitions; empty for a BOS edge.
#[inline]
fn transitions(lattice: &Lattice, slot: usize, edge_index: u32) -> &[PathEntry] {
    let paths = lattice.paths_at_char(slot);
    let start = paths.partition_point(|p| p.edge_index() < edge_index);
    let end = paths.partition_point(|p| p.edge_index() <= edge_index);
    &paths[start..end]
}

/// Returns the token of an edge of a path, with the ends of
/// [`Lattice::tokens_offset`]: the whitespace an entry ends with is part of
/// its token, skipped whitespace is not.
///
/// # Arguments
///
/// * `lattice` - The N-best lattice.
/// * `slot` - The slot holding the edge.
/// * `edge` - The edge.
///
/// # Returns
///
/// The token, or `None` for the EOS edge, the only edge whose start equals
/// the slot it is stored in (a zero-length span at the end of the sentence;
/// a carried edge always starts before its slot).
#[inline]
fn token(lattice: &Lattice, slot: usize, edge: &Edge) -> Option<TokenOffset> {
    let start = edge.start_char() as usize;
    (start != slot).then(|| {
        (
            lattice.byte_offset_of(start),
            lattice.edge_end_byte(edge, slot),
            edge.word_id(),
        )
    })
}

/// The end of a chain of [`NodeEdge`]s: the edges a search of
/// [`UniqueNBestGenerator`] starts from continue with nothing.
const NO_EDGE: usize = usize::MAX;

/// The initial capacity of the queue and the buffers of
/// [`UniqueNBestGenerator`]: enough for the few dozen nodes the search of a
/// short sentence takes, so they do not regrow there.
const INITIAL_CAPACITY: usize = 64;

/// Returns the key [`UniqueNBestGenerator`] groups the left edges of an
/// expansion by, all of one slot: within a slot, the start position and the
/// whitespace an entry ends with determine the token span (the slot fixes
/// where the whitespace before it starts), and a BOS edge is a node of its
/// own. Spans sort by start, then by end, before the BOS edges.
///
/// # Arguments
///
/// * `edge` - An edge of the slot.
/// * `index` - Its index in the slot.
///
/// # Returns
///
/// The key.
#[inline]
fn span_key(edge: &Edge, index: u32) -> u64 {
    if edge.left_index() == u32::MAX {
        1 << 40 | u64::from(index)
    } else {
        u64::from(edge.start_char()) << 8 | u64::from(edge.ws_tail())
    }
}

/// An edge of a node of [`UniqueNBestGenerator`], with the cheapest way from
/// it to the end of the search along the node's suffix of token spans.
#[derive(Clone, Copy, Debug)]
struct NodeEdge {
    /// The cost of that way, counted as `QueueElement::gx` counts it: from
    /// the end of the search up to, but not including, the edge's own word
    /// cost.
    gx: i64,
    /// The slot holding the edge.
    slot: u32,
    /// The index of the edge in its slot.
    edge_index: u32,
    /// The node edge (index in [`UniqueNBestGenerator::edges`]) the way
    /// continues with toward the end, or [`NO_EDGE`].
    next: usize,
}

/// A queued node of [`UniqueNBestGenerator`]: a token span (or a BOS edge)
/// followed by a suffix of token spans to the end of the search, as the
/// range of its edges in [`UniqueNBestGenerator::edges`].
#[derive(Clone, Copy, Debug)]
struct SpanNode {
    /// The cost of the cheapest path through the node, the smallest forward
    /// path cost plus `gx` of its edges: exact, as the forward pass gives the
    /// cheapest cost from BOS to every edge.
    fx: i64,
    /// Push order (see [`UniqueNBestGenerator::push_node`]).
    seq: u64,
    /// The first of the node's edges.
    start: usize,
    /// The end of the node's edges.
    end: usize,
}

/// Min-heap ordering, as for [`QueueElement`]: lower `fx` first, then the
/// node pushed later.
impl Ord for SpanNode {
    /// Orders by `fx` reversed, then by `seq`.
    ///
    /// # Arguments
    ///
    /// * `other` - The node to compare with.
    ///
    /// # Returns
    ///
    /// The ordering of `self` relative to `other`.
    fn cmp(&self, other: &Self) -> Ordering {
        other.fx.cmp(&self.fx).then(self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for SpanNode {
    /// The total order of `cmp`.
    ///
    /// # Arguments
    ///
    /// * `other` - The node to compare with.
    ///
    /// # Returns
    ///
    /// Always `Some`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for SpanNode {}

impl PartialEq for SpanNode {
    /// Compares the fields `cmp` orders by.
    ///
    /// # Arguments
    ///
    /// * `other` - The node to compare with.
    ///
    /// # Returns
    ///
    /// Whether both have the same `fx` and `seq`.
    fn eq(&self, other: &Self) -> bool {
        self.fx == other.fx && self.seq == other.seq
    }
}

/// The cheapest way found into one left edge while a node of
/// [`UniqueNBestGenerator`] is expanded.
#[derive(Clone, Copy, Debug)]
struct Way {
    /// The cost of the way, as [`NodeEdge::gx`].
    gx: i64,
    /// The node edge the way continues with.
    next: usize,
    /// The position of the transition among those the expansion scanned.
    order: u32,
}

/// Generates the N-best distinct segmentations of a lattice: for each
/// sequence of token spans (word boundaries) and BOS edge, its cheapest
/// path, in ascending order of cost, without enumerating the other paths of
/// the segmentation. This is what unique N-best needs
/// ([`Lattice::nbest_tokens_offset`] with `unique`): a segmentation can have
/// a number of paths exponential in its length, one per choice of entry for
/// each word (#1114).
///
/// The search runs backward from EOS (or from an exit) like
/// [`NBestGenerator`], but over suffixes of token spans instead of paths. A
/// node holds every edge of its first span, each with the cheapest way from
/// it to the end along the node's suffix. Expanding a node follows the
/// transitions of all its edges, keeps the cheapest way into each left
/// edge, and makes one child per left span (and one per BOS edge). The cost
/// of a node is the cheapest forward path cost plus way of its edges, which
/// is the cost of the cheapest path with that suffix, so the complete paths
/// come out in ascending order of cost, each (segmentation, BOS edge) pair
/// once. The work grows with the number of results and the length of the
/// sentence, not with the number of paths.
///
/// The first path is the lattice's best path, the one
/// [`Lattice::tokens_offset_into`] backtraces (from an exit,
/// [`Lattice::exit_tokens_offset_into`]), with its BOS index, as
/// [`NBestGenerator`] yields it first; the search skips its segmentation
/// once, so ties cannot change the first result. Of equal-cost ways into
/// one edge, the one the expansion scanned last wins, as [`NBestGenerator`]
/// pops the last pushed of equal-cost elements, and of equal-cost nodes the
/// one pushed last is expanded first. So identical entries (same surface,
/// context ids and cost) resolve to the first CSV row in every result, as in
/// the best path (#1131). Which of several equal-cost paths with different
/// context ids a result holds is deterministic but not otherwise specified.
///
/// The costs are exact unless a forward path cost was clamped at the
/// lattice's limit, which only absurd penalties reach.
pub struct UniqueNBestGenerator<'a> {
    /// The N-best lattice the paths are taken from.
    lattice: &'a Lattice,
    /// The first path and its BOS index, until the first call returns it.
    first: Option<(NBestPath, usize)>,
    /// The word boundaries and BOS index of the first path, which the
    /// search finds again and skips once.
    skip: Option<(Vec<(usize, usize)>, usize)>,
    /// The frontier, cheapest first.
    queue: BinaryHeap<SpanNode>,
    /// The edges of all nodes pushed so far; the nodes are ranges of it.
    edges: Vec<NodeEdge>,
    /// The number of nodes pushed so far, the next node's `seq`.
    pushed: u64,
    /// Scratch of an expansion: the cheapest way into each edge of the left
    /// slot, by edge index; `None` for the edges no transition reached.
    ways: Vec<Option<Way>>,
    /// Scratch of an expansion: the edges of the left slot with a way, as
    /// their [`span_key`] and index, in the order the transitions reached
    /// them.
    touched: Vec<(u64, u32)>,
    /// Scratch of an expansion: the children, as (cost, the position of the
    /// last transition that gives the cost, range of edges).
    children: Vec<(i64, u32, usize, usize)>,
    /// The number of nodes the tests let the search take from the queue
    /// before it panics: their bound on the work.
    #[cfg(test)]
    budget: usize,
}

impl<'a> UniqueNBestGenerator<'a> {
    /// Initializes the generator for the paths through EOS, with the EOS
    /// connection in their costs, as [`NBestGenerator::new`].
    ///
    /// # Arguments
    ///
    /// * `lattice` - The N-best lattice (`set_text_nbest*`).
    ///
    /// # Returns
    ///
    /// The generator; it yields nothing when the lattice holds no complete
    /// path.
    pub fn new(lattice: &'a Lattice) -> Self {
        let mut generator = Self::empty(lattice);
        let slot = lattice.char_len();
        let mut tokens = Vec::new();
        let (Some(eos_index), Some(bos)) =
            (lattice.eos_index(), lattice.tokens_offset_into(&mut tokens))
        else {
            return generator;
        };
        let cost = i64::from(lattice.edges_at_char(slot)[eos_index].path_cost());
        generator.set_first(tokens, cost, bos);
        generator.edges.push(NodeEdge {
            gx: 0,
            slot: slot as u32,
            edge_index: eos_index as u32,
            next: NO_EDGE,
        });
        generator.push_node(cost, 0, 1);
        generator
    }

    /// Initializes the generator for the paths that end with the right
    /// context id of `exit`, without EOS, as [`NBestGenerator::from_exit`]:
    /// the costs include the last word's Decompose length penalty but not
    /// the EOS connection, and the first path is the exit's own path at
    /// `exit.cost()`. For a sentence without words the paths are the empty
    /// ones of the BOS edges with that right id.
    ///
    /// # Arguments
    ///
    /// * `lattice` - The N-best lattice (`set_text_nbest*`).
    /// * `exit` - An exit of the lattice's current sentence, from
    ///   [`Lattice::exits_into`].
    ///
    /// # Returns
    ///
    /// The generator.
    ///
    /// # Panics
    ///
    /// If `exit` is not an exit of the lattice's current sentence, as
    /// [`Lattice::exit_tokens_offset_into`].
    pub fn from_exit(lattice: &'a Lattice, exit: &LatticeExit) -> Self {
        let mut generator = Self::empty(lattice);
        let mut tokens = Vec::new();
        let bos = lattice.exit_tokens_offset_into(exit, &mut tokens);
        generator.set_first(tokens, i64::from(exit.cost()), bos);
        let slot = lattice.char_len();
        generator.reset_ways(lattice.edges_at_char(slot).len());
        for (i, edge) in lattice.final_edges().iter().enumerate() {
            if edge.right_id() != exit.right_id() {
                continue;
            }
            let way = Way {
                gx: i64::from(lattice.exit_penalty(i)),
                next: NO_EDGE,
                order: i as u32,
            };
            generator.offer(i as u32, edge, way);
        }
        generator.push_children(slot);
        generator
    }

    /// Returns a generator with an empty queue.
    ///
    /// # Arguments
    ///
    /// * `lattice` - The N-best lattice.
    ///
    /// # Returns
    ///
    /// The generator; it yields nothing.
    fn empty(lattice: &'a Lattice) -> Self {
        Self {
            lattice,
            first: None,
            skip: None,
            // Room for the search of a short sentence without regrowing.
            queue: BinaryHeap::with_capacity(INITIAL_CAPACITY),
            edges: Vec::with_capacity(INITIAL_CAPACITY),
            pushed: 0,
            ways: Vec::new(),
            touched: Vec::with_capacity(INITIAL_CAPACITY),
            children: Vec::new(),
            #[cfg(test)]
            budget: usize::MAX,
        }
    }

    /// Bounds the number of nodes the search may take from the queue: a
    /// test's check that the work stays proportional to what it expects,
    /// which fails at once instead of running out of memory.
    ///
    /// # Arguments
    ///
    /// * `budget` - The number of nodes.
    ///
    /// # Returns
    ///
    /// The generator.
    #[cfg(test)]
    pub(crate) fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    /// Sets the first path, and its word boundaries as the ones to skip.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The tokens of the lattice's best path.
    /// * `cost` - Its cost.
    /// * `bos` - Its BOS index.
    fn set_first(&mut self, tokens: Vec<TokenOffset>, cost: i64, bos: usize) {
        let bounds = tokens.iter().map(|&(start, end, _)| (start, end)).collect();
        self.skip = Some((bounds, bos));
        self.first = Some(((tokens, cost), bos));
    }

    /// Pushes a node, numbering it in push order: of the nodes with the
    /// same cost, the last pushed is expanded first, as in
    /// [`NBestGenerator`].
    ///
    /// # Arguments
    ///
    /// * `fx` - The cost of the cheapest path through the node.
    /// * `start` - The first of its edges in `edges`.
    /// * `end` - The end of its edges.
    fn push_node(&mut self, fx: i64, start: usize, end: usize) {
        let seq = self.pushed;
        self.pushed += 1;
        self.queue.push(SpanNode {
            fx,
            seq,
            start,
            end,
        });
    }

    /// Makes room in the scratch of an expansion for the edges of the left
    /// slot; the entries a previous expansion set were cleared by
    /// [`UniqueNBestGenerator::push_children`].
    ///
    /// # Arguments
    ///
    /// * `len` - The number of edges in the left slot.
    fn reset_ways(&mut self, len: usize) {
        if self.ways.len() < len {
            self.ways.resize(len, None);
        }
    }

    /// Offers a way into an edge of the left slot: it replaces the edge's
    /// way unless that one is cheaper, so of equal-cost ways the later one
    /// wins.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the edge in the left slot.
    /// * `edge` - The edge.
    /// * `way` - The way.
    fn offer(&mut self, index: u32, edge: &Edge, way: Way) {
        match &mut self.ways[index as usize] {
            Some(best) => {
                if way.gx <= best.gx {
                    *best = way;
                }
            }
            empty => {
                *empty = Some(way);
                self.touched.push((span_key(edge, index), index));
            }
        }
    }

    /// Turns the ways of an expansion into child nodes and pushes them: one
    /// node per token span of the left slot, one per BOS edge. The children
    /// are pushed in the order of the last transition that gives each its
    /// cost, so of equal-cost children the one reached last is expanded
    /// first, as [`NBestGenerator`] expands the last pushed of equal-cost
    /// predecessors first. Clears the scratch for the next expansion.
    ///
    /// # Arguments
    ///
    /// * `left_slot` - The slot the ways lead into.
    fn push_children(&mut self, left_slot: usize) {
        let lattice = self.lattice;
        let left_edges = lattice.edges_at_char(left_slot);
        if let [(_, index)] = self.touched[..] {
            // One left edge, one child: nothing to group or order.
            self.touched.clear();
            if let Some(way) = self.ways[index as usize].take() {
                let start = self.edges.len();
                self.edges.push(NodeEdge {
                    gx: way.gx,
                    slot: left_slot as u32,
                    edge_index: index,
                    next: way.next,
                });
                let fx = i64::from(left_edges[index as usize].path_cost()) + way.gx;
                self.push_node(fx, start, start + 1);
            }
            return;
        }
        self.touched.sort_unstable();

        self.children.clear();
        let mut i = 0;
        while i < self.touched.len() {
            let group = self.touched[i].0;
            let start = self.edges.len();
            let mut fx = i64::MAX;
            let mut position = 0;
            while i < self.touched.len() && self.touched[i].0 == group {
                let index = self.touched[i].1;
                i += 1;
                let Some(way) = self.ways[index as usize].take() else {
                    continue;
                };
                let cost = i64::from(left_edges[index as usize].path_cost()) + way.gx;
                if cost < fx || (cost == fx && way.order > position) {
                    position = way.order;
                }
                fx = fx.min(cost);
                self.edges.push(NodeEdge {
                    gx: way.gx,
                    slot: left_slot as u32,
                    edge_index: index,
                    next: way.next,
                });
            }
            self.children.push((fx, position, start, self.edges.len()));
        }
        self.touched.clear();

        if self.children.len() > 1 {
            self.children
                .sort_unstable_by_key(|&(_, position, _, _)| position);
        }
        for k in 0..self.children.len() {
            let (fx, _, start, end) = self.children[k];
            self.push_node(fx, start, end);
        }
    }

    /// Expands a node: offers the ways into the left edges of all its edges,
    /// then pushes the children.
    ///
    /// # Arguments
    ///
    /// * `node` - The node, not a BOS edge.
    fn expand(&mut self, node: SpanNode) {
        let lattice = self.lattice;
        let slot = self.edges[node.start].slot as usize;
        let slot_edges = lattice.edges_at_char(slot);
        // All edges of the node share their start, the slot their
        // transitions come from.
        let left_slot =
            slot_edges[self.edges[node.start].edge_index as usize].start_char() as usize;
        let left_edges = lattice.edges_at_char(left_slot);
        self.reset_ways(left_edges.len());
        let mut order = 0;
        for index in node.start..node.end {
            let node_edge = self.edges[index];
            let edge = &slot_edges[node_edge.edge_index as usize];
            debug_assert_eq!(edge.start_char() as usize, left_slot);
            for entry in transitions(lattice, slot, node_edge.edge_index) {
                debug_assert_eq!(entry.left_pos() as usize, left_slot);
                let Some(left_edge) = left_edges.get(entry.left_index() as usize) else {
                    continue;
                };
                // As in `NBestGenerator::next_with_bos`: the entry's cost is
                // the left edge's path cost plus the connection and penalty.
                let gx = node_edge.gx + i64::from(entry.cost()) - i64::from(left_edge.path_cost())
                    + i64::from(edge.word_cost());
                self.offer(
                    entry.left_index(),
                    left_edge,
                    Way {
                        gx,
                        next: index,
                        order,
                    },
                );
                order += 1;
            }
        }
        self.push_children(left_slot);
    }

    /// Returns whether a complete path has the word boundaries and BOS index
    /// of the first path, which the search skips once, comparing the chain
    /// of node edges with them without building the tokens.
    ///
    /// # Arguments
    ///
    /// * `bos` - The BOS index of the path.
    /// * `next` - The node edge after its BOS edge.
    ///
    /// # Returns
    ///
    /// `true` for the first path's pair while it has not been skipped.
    fn is_first(&self, bos: usize, mut next: usize) -> bool {
        let Some((bounds, first_bos)) = &self.skip else {
            return false;
        };
        if *first_bos != bos {
            return false;
        }
        let mut bounds = bounds.iter();
        while next != NO_EDGE {
            let node_edge = &self.edges[next];
            let slot = node_edge.slot as usize;
            let edge = &self.lattice.edges_at_char(slot)[node_edge.edge_index as usize];
            if let Some((start, end, _)) = token(self.lattice, slot, edge)
                && bounds.next() != Some(&(start, end))
            {
                return false;
            }
            next = node_edge.next;
        }
        bounds.next().is_none()
    }

    /// Rebuilds the tokens of a complete path from the chain of node edges
    /// that follows its BOS edge.
    ///
    /// # Arguments
    ///
    /// * `next` - The node edge after the BOS edge.
    ///
    /// # Returns
    ///
    /// The tokens in reading order; empty for a path without words.
    fn reconstruct(&self, mut next: usize) -> Vec<TokenOffset> {
        let mut path = Vec::new();
        while next != NO_EDGE {
            let node_edge = &self.edges[next];
            let slot = node_edge.slot as usize;
            let edge = &self.lattice.edges_at_char(slot)[node_edge.edge_index as usize];
            if let Some(token) = token(self.lattice, slot, edge) {
                path.push(token);
            }
            next = node_edge.next;
        }
        path
    }

    /// Returns the next segmentation as its cheapest path and that path's
    /// cost, as [`NBestGenerator::next`].
    ///
    /// # Returns
    ///
    /// The next path and its cost, or `None` when there are no more; see
    /// [`UniqueNBestGenerator::next_with_bos`].
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<NBestPath> {
        self.next_with_bos().map(|(path, _)| path)
    }

    /// Returns the next segmentation with its BOS index, as
    /// [`NBestGenerator::next_with_bos`]: the cheapest path of the next
    /// (word boundaries, BOS edge) pair in ascending order of cost.
    ///
    /// # Returns
    ///
    /// The path, its cost (the BOS edge's cost included) and its BOS index,
    /// or `None` when there are no more.
    pub fn next_with_bos(&mut self) -> Option<(NBestPath, usize)> {
        if let Some(first) = self.first.take() {
            return Some(first);
        }
        while let Some(node) = self.queue.pop() {
            #[cfg(test)]
            {
                assert!(self.budget > 0, "the search exceeded its node budget");
                self.budget -= 1;
            }
            let node_edge = self.edges[node.start];
            let edge =
                &self.lattice.edges_at_char(node_edge.slot as usize)[node_edge.edge_index as usize];
            if edge.left_index() != u32::MAX {
                self.expand(node);
                continue;
            }
            // A BOS edge: a complete path.
            let bos = self.lattice.bos_index(node_edge.edge_index as usize);
            if self.is_first(bos, node_edge.next) {
                self.skip = None;
                continue;
            }
            return Some(((self.reconstruct(node_edge.next), node.fx), bos));
        }
        None
    }
}

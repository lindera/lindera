//! The paths that the 1-best segmentation keeps open across the cuts that
//! carry the context (#1096).
//!
//! Inside a segment (a run of sentences that carry the context from one to
//! the next), the best path is not known until the segment ends: every exit
//! of a sentence (a right context id of the words that end it) is a state
//! with its own best path so far. [`PathTree`] stores those paths as a tree
//! of per-sentence parts, so the states share the parts they have in
//! common. Once all the states descend from one part, every part up to it
//! is final; the segmenter emits them and the tree drops them, which keeps
//! the tree small: after `、` the states almost always meet within one
//! sentence.

use lindera_dictionary::viterbi::TokenOffset;

/// The index that stands for no node: the parent of a part that starts
/// right after the emitted part of the segment, and the node of a state
/// whose path is emitted in full.
pub(super) const NO_NODE: u32 = u32::MAX;

/// One state of the 1-best search across carried cuts: the best path so far
/// that ends with one right context id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CarryState {
    /// The right context id of the last word of the path (of the BOS edge
    /// for a path without words), the context the next sentence starts
    /// from.
    pub(super) right_id: u16,
    /// The cost of the path from the start of the segment, without the EOS
    /// connection.
    pub(super) cost: i64,
    /// The node of the path's last part in the [`PathTree`], or [`NO_NODE`]
    /// when the whole path is emitted (or empty).
    pub(super) node: u32,
    /// The edge index of the state's exit (`LatticeExit::edge_index`): of
    /// two states that lead to paths of equal cost, one lattice over the
    /// line keeps the one with the greater rank (#1140). 0 for a segment's
    /// first state.
    pub(super) rank: u32,
}

impl CarryState {
    /// The state a segment starts from: the dictionary's BOS context (right
    /// id 0) at cost 0, with an empty path.
    pub(super) const ROOT: CarryState = CarryState {
        right_id: 0,
        cost: 0,
        node: NO_NODE,
        rank: 0,
    };
}

/// One sentence's part of an open path.
#[derive(Debug, Default)]
struct PathNode {
    /// The node of the part before this one, or [`NO_NODE`].
    parent: u32,
    /// The start of the part's sentence in the input, in bytes.
    base: usize,
    /// The part's tokens, with offsets relative to `base`.
    tokens: Vec<TokenOffset>,
}

/// The open paths of the 1-best segmentation, as a tree of per-sentence
/// parts. Kept by `SegmentBuffers`, so its buffers are reused across
/// sentences and calls.
#[derive(Debug, Default)]
pub(crate) struct PathTree {
    /// The nodes of the open paths.
    nodes: Vec<PathNode>,
    /// The nodes kept by [`PathTree::retain`], built from `nodes` and then
    /// swapped with it.
    kept: Vec<PathNode>,
    /// Token buffers of dropped nodes, reused by [`PathTree::add`].
    spare: Vec<Vec<TokenOffset>>,
    /// Scratch of [`PathTree::retain`]: the new index of every node, or
    /// [`NO_NODE`].
    remap: Vec<u32>,
    /// Scratch of [`PathTree::retain`] and [`PathTree::for_each_part`]: a
    /// chain of nodes.
    stack: Vec<u32>,
}

impl PathTree {
    /// Adds a node without a parent (see [`PathTree::set_parent`]) and
    /// without tokens (see [`PathTree::tokens_mut`]).
    ///
    /// # 引数
    ///
    /// * `base` - The start of the node's sentence in the input, in bytes.
    ///
    /// # 戻り値
    ///
    /// The new node.
    pub(super) fn add(&mut self, base: usize) -> u32 {
        let tokens = self.spare.pop().unwrap_or_default();
        self.nodes.push(PathNode {
            parent: NO_NODE,
            base,
            tokens,
        });
        (self.nodes.len() - 1) as u32
    }

    /// Returns the token buffer of a node, to fill with its tokens.
    ///
    /// # 引数
    ///
    /// * `node` - A node of the tree.
    ///
    /// # 戻り値
    ///
    /// The buffer; its offsets are relative to the node's `base`.
    pub(super) fn tokens_mut(&mut self, node: u32) -> &mut Vec<TokenOffset> {
        &mut self.nodes[node as usize].tokens
    }

    /// Sets the parent of a node.
    ///
    /// # 引数
    ///
    /// * `node` - A node of the tree.
    /// * `parent` - The node of the part before it, or [`NO_NODE`].
    pub(super) fn set_parent(&mut self, node: u32, parent: u32) {
        self.nodes[node as usize].parent = parent;
    }

    /// Returns the parent of a node.
    ///
    /// # 引数
    ///
    /// * `node` - A node of the tree, or [`NO_NODE`].
    ///
    /// # 戻り値
    ///
    /// The parent, or [`NO_NODE`] for a node without one and for
    /// [`NO_NODE`] itself.
    #[inline]
    fn parent(&self, node: u32) -> u32 {
        if node == NO_NODE {
            NO_NODE
        } else {
            self.nodes[node as usize].parent
        }
    }

    /// Calls `f` with every part of the path that ends with `node`, in
    /// reading order.
    ///
    /// # 引数
    ///
    /// * `node` - The last part of the path, or [`NO_NODE`] for nothing.
    /// * `f` - Called with each part's sentence start and tokens.
    pub(super) fn for_each_part(&mut self, node: u32, mut f: impl FnMut(usize, &[TokenOffset])) {
        self.stack.clear();
        let mut current = node;
        while current != NO_NODE {
            self.stack.push(current);
            current = self.nodes[current as usize].parent;
        }
        for &index in self.stack.iter().rev() {
            let node = &self.nodes[index as usize];
            f(node.base, &node.tokens);
        }
    }

    /// Returns the last part that the paths of all `states` share.
    ///
    /// The nodes of the states are all at the same depth (every sentence of
    /// a segment adds one level), so the paths are climbed in lock step.
    ///
    /// # 引数
    ///
    /// * `states` - The states; not empty.
    ///
    /// # 戻り値
    ///
    /// The deepest common node, or [`NO_NODE`] when the paths share no
    /// open part.
    pub(super) fn common_ancestor(&self, states: &[CarryState]) -> u32 {
        let Some((first, rest)) = states.split_first() else {
            return NO_NODE;
        };
        let mut common = first.node;
        // How many levels `common` is above the states' nodes.
        let mut climbed = 0;
        for state in rest {
            let mut node = state.node;
            for _ in 0..climbed {
                node = self.parent(node);
            }
            while node != common {
                node = self.parent(node);
                common = self.parent(common);
                climbed += 1;
            }
        }
        common
    }

    /// Keeps only the nodes on the paths of `states`, below `cut`, and
    /// renumbers them; the token buffers of the other nodes become spare.
    ///
    /// # 引数
    ///
    /// * `states` - The states; their nodes are renumbered, and a state
    ///   whose node is `cut` gets [`NO_NODE`].
    /// * `cut` - The node up to which the paths were emitted (its children
    ///   lose their parent), or [`NO_NODE`] when nothing was emitted.
    pub(super) fn retain(&mut self, states: &mut [CarryState], cut: u32) {
        let Self {
            nodes,
            kept,
            spare,
            remap,
            stack,
        } = self;
        remap.clear();
        remap.resize(nodes.len(), NO_NODE);
        kept.clear();
        for state in states.iter_mut() {
            stack.clear();
            let mut current = state.node;
            while current != NO_NODE && current != cut && remap[current as usize] == NO_NODE {
                stack.push(current);
                current = nodes[current as usize].parent;
            }
            let mut parent = if current == NO_NODE || current == cut {
                NO_NODE
            } else {
                remap[current as usize]
            };
            for &index in stack.iter().rev() {
                let node = &mut nodes[index as usize];
                kept.push(PathNode {
                    parent,
                    base: node.base,
                    tokens: std::mem::take(&mut node.tokens),
                });
                parent = (kept.len() - 1) as u32;
                remap[index as usize] = parent;
            }
            state.node = if state.node == NO_NODE || state.node == cut {
                NO_NODE
            } else {
                remap[state.node as usize]
            };
        }
        for node in nodes.drain(..) {
            Self::recycle(spare, node.tokens);
        }
        std::mem::swap(nodes, kept);
    }

    /// Drops every node, keeping the token buffers as spare.
    pub(super) fn clear(&mut self) {
        for node in self.nodes.drain(..) {
            Self::recycle(&mut self.spare, node.tokens);
        }
    }

    /// Returns a token buffer to the spare buffers, unless it holds no
    /// allocation.
    ///
    /// # 引数
    ///
    /// * `spare` - The spare buffers.
    /// * `tokens` - The buffer.
    fn recycle(spare: &mut Vec<Vec<TokenOffset>>, mut tokens: Vec<TokenOffset>) {
        if tokens.capacity() > 0 {
            tokens.clear();
            spare.push(tokens);
        }
    }

    /// Returns the number of nodes, for tests.
    ///
    /// # 戻り値
    ///
    /// The number of open parts.
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Releases the memory of every buffer, dropping the nodes.
    pub(crate) fn shrink_to_fit(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use lindera_dictionary::viterbi::WordId;

    use super::{CarryState, NO_NODE, PathTree};

    fn state(node: u32) -> CarryState {
        CarryState {
            right_id: 0,
            cost: 0,
            node,
            rank: 0,
        }
    }

    fn add(tree: &mut PathTree, parent: u32, base: usize, len: usize) -> u32 {
        let node = tree.add(base);
        tree.set_parent(node, parent);
        tree.tokens_mut(node)
            .extend((0..len).map(|i| (i, i + 1, WordId::default())));
        node
    }

    fn parts(tree: &mut PathTree, node: u32) -> Vec<(usize, usize)> {
        let mut parts = Vec::new();
        tree.for_each_part(node, |base, tokens| parts.push((base, tokens.len())));
        parts
    }

    #[test]
    fn test_common_ancestor_and_retain() {
        let mut tree = PathTree::default();
        // Level 1: a, b; level 2: c, d from a, e from b; level 3: f from c,
        // g from d.
        let a = add(&mut tree, NO_NODE, 0, 1);
        let b = add(&mut tree, NO_NODE, 0, 2);
        let c = add(&mut tree, a, 10, 3);
        let d = add(&mut tree, a, 10, 4);
        let e = add(&mut tree, b, 10, 5);
        assert_eq!(tree.common_ancestor(&[state(c), state(d)]), a);
        assert_eq!(tree.common_ancestor(&[state(c), state(e)]), NO_NODE);
        assert_eq!(
            tree.common_ancestor(&[state(c), state(d), state(e)]),
            NO_NODE
        );
        assert_eq!(tree.common_ancestor(&[state(e)]), e);

        let f = add(&mut tree, c, 20, 6);
        let g = add(&mut tree, d, 20, 7);
        let mut states = [state(f), state(g)];
        assert_eq!(tree.common_ancestor(&states), a);
        assert_eq!(parts(&mut tree, f), vec![(0, 1), (10, 3), (20, 6)]);

        // Emitting `a` leaves c, d, f, g; b and e are dropped.
        tree.retain(&mut states, a);
        assert_eq!(tree.len(), 4);
        assert_eq!(parts(&mut tree, states[0].node), vec![(10, 3), (20, 6)]);
        assert_eq!(parts(&mut tree, states[1].node), vec![(10, 4), (20, 7)]);

        // A single state is its own common ancestor and empties the tree.
        let mut single = [states[1]];
        let cut = tree.common_ancestor(&single);
        assert_eq!(cut, single[0].node);
        tree.retain(&mut single, cut);
        assert_eq!(tree.len(), 0);
        assert_eq!(single[0].node, NO_NODE);
        assert!(parts(&mut tree, NO_NODE).is_empty());

        // The dropped buffers are reused.
        let spare = tree.spare.len();
        assert!(spare > 0);
        let h = add(&mut tree, NO_NODE, 30, 1);
        assert_eq!(tree.spare.len(), spare - 1);
        tree.clear();
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.spare.len(), spare);
        let _ = h;
    }

    #[test]
    fn test_retain_without_cut_shares_parents() {
        let mut tree = PathTree::default();
        let a = add(&mut tree, NO_NODE, 0, 1);
        let b = add(&mut tree, a, 5, 2);
        let c = add(&mut tree, a, 5, 3);
        let _dead = add(&mut tree, a, 5, 4);
        let mut states = [state(b), state(c)];
        tree.retain(&mut states, NO_NODE);
        // a, b, c are kept, a once.
        assert_eq!(tree.len(), 3);
        assert_eq!(parts(&mut tree, states[0].node), vec![(0, 1), (5, 2)]);
        assert_eq!(parts(&mut tree, states[1].node), vec![(0, 1), (5, 3)]);
        assert_eq!(tree.common_ancestor(&states), 0);
    }
}

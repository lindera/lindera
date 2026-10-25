use core::num::NonZeroU32;

use alloc::vec::Vec;

use crate::errors::{Result, RucrfError};

/// Represents an edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    target: usize,
    pub(crate) label: NonZeroU32,
}

impl Edge {
    /// Creates a new edge.
    #[inline(always)]
    #[must_use]
    pub const fn new(target: usize, label: NonZeroU32) -> Self {
        Self { target, label }
    }

    /// Returns an index of the target node.
    #[inline(always)]
    #[must_use]
    pub const fn target(&self) -> usize {
        self.target
    }

    /// Returns a label of this edge.
    #[inline(always)]
    #[must_use]
    pub const fn label(&self) -> NonZeroU32 {
        self.label
    }
}

/// Represents a node.
#[derive(Clone, Default, Debug)]
pub struct Node {
    edges: Vec<Edge>,
}

impl Node {
    /// Returns a list of edges.
    ///
    /// In training, the first edge is treated as a positive example.
    #[inline(always)]
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

/// Represents a lattice.
pub struct Lattice {
    nodes: Vec<Node>,
}

impl Lattice {
    /// Creates a new lattice.
    ///
    /// # Arguments
    ///
    /// * `length` - The length of this lattice.
    ///
    /// # Errors
    ///
    /// `length` must be >= 1.
    #[inline(always)]
    pub fn new(length: usize) -> Result<Self> {
        if length == 0 {
            return Err(RucrfError::invalid_argument("length must be >= 1"));
        }
        let nodes = vec![Node::default(); length + 1];
        Ok(Self { nodes })
    }

    /// Adds a new edge.
    ///
    /// In training, the first edge of each position is treated as the positive example.
    ///
    /// # Errors
    ///
    /// `edge.target()` must be > `pos` and <= `length`.
    #[inline(always)]
    pub fn add_edge(&mut self, pos: usize, edge: Edge) -> Result<()> {
        if edge.target() <= pos {
            return Err(RucrfError::invalid_argument("edge.target() must be > pos"));
        }
        // The nodes are 0..=length.
        if edge.target() >= self.nodes.len() {
            return Err(RucrfError::invalid_argument(
                "edge.target() must be <= length",
            ));
        }
        self.nodes[pos].edges.push(edge);
        Ok(())
    }

    /// Returns a list of nodes.
    #[inline(always)]
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label() -> NonZeroU32 {
        NonZeroU32::new(1).unwrap()
    }

    // A lattice of length n has the nodes 0..=n, so no edge ends past n; an
    // edge to n + 1 made training and search_best_path index past the last
    // node (#1264).
    #[test]
    fn add_edge_rejects_a_target_past_the_length() {
        for length in 1..=3 {
            for target in [length + 1, length + 2, usize::MAX] {
                let mut lattice = Lattice::new(length).unwrap();
                let result = lattice.add_edge(0, Edge::new(target, label()));
                assert!(
                    matches!(result, Err(RucrfError::InvalidArgument(_))),
                    "length {length}, target {target}: {result:?}"
                );
                assert!(
                    lattice.nodes()[0].edges().is_empty(),
                    "length {length}, target {target}"
                );
            }
        }
    }

    #[test]
    fn add_edge_accepts_every_target_after_pos_up_to_the_length() {
        for length in 1..=3 {
            let mut lattice = Lattice::new(length).unwrap();
            for pos in 0..length {
                for target in pos + 1..=length {
                    lattice.add_edge(pos, Edge::new(target, label())).unwrap();
                }
            }
            for pos in 0..length {
                let targets: Vec<usize> = lattice.nodes()[pos]
                    .edges()
                    .iter()
                    .map(Edge::target)
                    .collect();
                assert_eq!(
                    targets,
                    (pos + 1..=length).collect::<Vec<_>>(),
                    "length {length}, pos {pos}"
                );
            }
            assert!(lattice.nodes()[length].edges().is_empty());
        }
    }

    #[test]
    fn add_edge_rejects_a_target_not_after_pos() {
        let mut lattice = Lattice::new(3).unwrap();
        for (pos, target) in [(1, 1), (2, 1), (1, 0)] {
            let result = lattice.add_edge(pos, Edge::new(target, label()));
            assert!(
                matches!(result, Err(RucrfError::InvalidArgument(_))),
                "pos {pos}, target {target}: {result:?}"
            );
        }
        assert!(lattice.nodes().iter().all(|node| node.edges().is_empty()));
    }
}

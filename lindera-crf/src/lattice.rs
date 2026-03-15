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
    /// `edge.target()` must be >= `pos` and <= `length`.
    #[inline(always)]
    pub fn add_edge(&mut self, pos: usize, edge: Edge) -> Result<()> {
        if edge.target() <= pos {
            return Err(RucrfError::invalid_argument("edge.target() must be > pos"));
        }
        if edge.target() > self.nodes.len() {
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

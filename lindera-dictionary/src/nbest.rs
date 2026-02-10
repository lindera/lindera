use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::viterbi::{Lattice, WordId};

/// An element in the A* priority queue for N-Best search.
/// Represents a partial path from EOS backward toward BOS.
#[derive(Clone, Debug)]
struct QueueElement {
    /// Byte position of the current edge in ends_at
    byte_pos: u32,
    /// Index of the current edge in ends_at[byte_pos]
    edge_index: u16,
    /// f(x) = g(x) + h(x) -- total estimated cost
    fx: i64,
    /// g(x) = accumulated real cost from EOS backward to this point
    gx: i64,
    /// Link to the previous QueueElement in the elements chain (toward EOS)
    prev: Option<usize>,
}

/// Min-heap ordering: lower fx = higher priority
impl Ord for QueueElement {
    fn cmp(&self, other: &Self) -> Ordering {
        other.fx.cmp(&self.fx) // Reversed for min-heap
    }
}

impl PartialOrd for QueueElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for QueueElement {}

impl PartialEq for QueueElement {
    fn eq(&self, other: &Self) -> bool {
        self.fx == other.fx
    }
}

/// Generates N-best paths through a Lattice using Backward A* search.
///
/// After forward Viterbi (set_text_nbest), this generator uses the recorded
/// all_paths transitions and path_cost heuristics to enumerate paths
/// from EOS to BOS in order of increasing total cost.
pub struct NBestGenerator<'a> {
    lattice: &'a Lattice,
    queue: BinaryHeap<QueueElement>,
    /// Storage for QueueElement chain (for path reconstruction)
    elements: Vec<QueueElement>,
}

impl<'a> NBestGenerator<'a> {
    /// Initialize the generator from a lattice that has been processed
    /// with set_text_nbest().
    pub fn new(lattice: &'a Lattice) -> Self {
        let mut generator = NBestGenerator {
            lattice,
            queue: BinaryHeap::new(),
            elements: Vec::new(),
        };
        generator.init();
        generator
    }

    fn init(&mut self) {
        let text_len = self.lattice.text_len();
        let eos_edges = self.lattice.edges_at(text_len);
        if eos_edges.is_empty() {
            return;
        }

        // EOS is the last edge pushed to ends_at[text_len]
        let eos_index = (eos_edges.len() - 1) as u16;
        let eos_edge = &eos_edges[eos_index as usize];

        // Initial element: start from EOS with g(x)=0
        let elem = QueueElement {
            byte_pos: text_len as u32,
            edge_index: eos_index,
            fx: eos_edge.path_cost as i64,
            gx: 0,
            prev: None,
        };
        self.queue.push(elem);
    }

    /// Returns the next best path as (path, cost).
    /// The path is a vector of (byte_start, WordId) pairs.
    /// The cost is the total path cost (fx at BOS), lower is better.
    /// Returns None when no more paths are available.
    pub fn next(&mut self) -> Option<(Vec<(usize, WordId)>, i64)> {
        while let Some(current) = self.queue.pop() {
            let byte_pos = current.byte_pos as usize;
            let edge_index = current.edge_index as usize;

            let edges = self.lattice.edges_at(byte_pos);
            if edge_index >= edges.len() {
                continue;
            }
            let edge = &edges[edge_index];

            // Check if we reached BOS (left_index == u16::MAX means no predecessor = BOS)
            if edge.left_index == u16::MAX {
                return Some((self.reconstruct_path(&current), current.fx));
            }

            // Store current element for chain linking
            let current_idx = self.elements.len();
            self.elements.push(current.clone());

            // Expand: for each predecessor path of this edge
            let paths = self.lattice.paths_at(byte_pos);
            for path_entry in paths {
                if path_entry.edge_index != edge_index as u16 {
                    continue; // Not a path to this edge
                }

                let left_pos = path_entry.left_pos as usize;
                let left_index = path_entry.left_index as usize;

                let left_edges = self.lattice.edges_at(left_pos);
                if left_index >= left_edges.len() {
                    continue;
                }
                let left_edge = &left_edges[left_index];

                // g(x) for the predecessor:
                // path_entry.cost = left_edge.path_cost + conn_cost + penalty
                // conn_and_penalty = path_entry.cost - left_edge.path_cost
                // new_gx = current.gx + conn_and_penalty + edge.word_cost
                let conn_and_penalty = path_entry.cost as i64 - left_edge.path_cost as i64;
                let new_gx = current.gx + conn_and_penalty + edge.word_entry.word_cost as i64;

                // f(x) = h(x) + g(x), where h(x) = left_edge.path_cost
                let new_fx = left_edge.path_cost as i64 + new_gx;

                let new_elem = QueueElement {
                    byte_pos: left_pos as u32,
                    edge_index: left_index as u16,
                    fx: new_fx,
                    gx: new_gx,
                    prev: Some(current_idx),
                };
                self.queue.push(new_elem);
            }
        }
        None
    }

    fn reconstruct_path(&self, bos_elem: &QueueElement) -> Vec<(usize, WordId)> {
        let mut path = Vec::new();
        let mut maybe_idx = bos_elem.prev;

        // Walk the chain from BOS toward EOS via prev links.
        // The chain visits edges in forward order (first word first)
        // because each element's prev points toward EOS.
        while let Some(idx) = maybe_idx {
            let elem = &self.elements[idx];
            let edges = self.lattice.edges_at(elem.byte_pos as usize);
            let edge = &edges[elem.edge_index as usize];

            // Skip EOS edge (start_index == stop_index)
            if edge.start_index != edge.stop_index {
                path.push((edge.start_index as usize, edge.word_entry.word_id));
            }

            maybe_idx = elem.prev;
        }

        path
    }
}

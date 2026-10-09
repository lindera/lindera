//! The N-best search of a segment whose sentences carry the context from
//! one to the next (#1096).
//!
//! A segment is a run of sentences cut at `、`, `。` or a forced cut and
//! ended by `\n`, `\t` or the end of the input. Its paths cost what one
//! lattice over the whole segment would give them: the EOS connection is
//! paid only at the end of the segment, and a sentence starts from the
//! exits of the previous one (see
//! [`LatticeOptions::bos`](lindera_dictionary::viterbi::LatticeOptions::bos)).
//!
//! [`CarriedNbest`] runs a DP over the sentences of one segment. A state is
//! an exit of the last sentence (a right context id of the words that end
//! it) and keeps its `n` cheapest prefixes (paths from the segment start to
//! the end of that sentence that end with its right id). The next sentence
//! is one N-best lattice with one BOS edge per state; for each of its exits,
//! [`merge_carried`] combines the paths that the lattice yields for the exit
//! with the prefixes of the states they start from. At the end of the
//! segment the same merge over the paths through EOS gives the segment's
//! results.
//!
//! Keeping `n` prefixes per state is exact: a result whose prefix is not
//! among the `n` cheapest (distinct, with `unique`) prefixes of its state
//! has `n` cheaper (distinct) results that share its suffix. The cost
//! threshold is exact per state as well: the cheapest completion of a
//! prefix depends only on its state, so a prefix more than the threshold
//! above its state's best cannot complete within the threshold of the
//! segment's best.
//!
//! An exit is skipped altogether when another state's `n`-th prefix beats
//! its best prefix whatever follows (see `exit_dominance`): the cost gap
//! exceeds the margin between their right ids, so every result through the
//! exit has `n` cheaper (distinct) results through the other state.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use lindera_dictionary::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use lindera_dictionary::nbest::NBestGenerator;
use lindera_dictionary::viterbi::{BosContext, Lattice, LatticeExit, NBestPath, TokenOffset};

use super::exit_dominance::MarginCache;
use super::fill_bos_contexts;
use super::nbest_merge::{CarriedPaths, merge_carried};

/// The index that stands for no prefix: the parent of the root prefix.
const NO_PREFIX: u32 = u32::MAX;

/// A prefix of the segment: a path from the segment start to the end of a
/// sentence, stored as the prefix it extends and this sentence's part.
#[derive(Clone, Copy, Debug)]
struct Prefix {
    /// The prefix this one extends, or [`NO_PREFIX`] for the root.
    parent: u32,
    /// The start of the part's sentence in the input, in bytes.
    base: usize,
    /// The part's tokens: a range of [`CarriedNbest::tokens`], with offsets
    /// relative to `base`.
    tokens: (u32, u32),
    /// With `unique`, the class of the prefix's word boundaries: two
    /// prefixes of the same sentence have the same class exactly when their
    /// boundaries are equal. 0 otherwise.
    class: u32,
}

/// A state: the prefixes that end with one right context id.
#[derive(Clone, Copy, Debug)]
struct State {
    /// The right context id of the last word of the prefixes, the context
    /// of the state's BOS edge in the next sentence.
    right_id: u16,
    /// The index of the state's first prefix in [`CarriedNbest::prefixes`];
    /// the prefixes of a state are contiguous and in ascending order of
    /// cost.
    start: u32,
    /// The number of prefixes of the state; at least 1.
    len: u32,
    /// The edge index of the state's exit (`LatticeExit::edge_index`): of
    /// two states that lead to paths of equal cost, one lattice over the
    /// line keeps the one with the greater rank (#1140). 0 for a segment's
    /// first state.
    rank: u32,
}

impl State {
    /// Returns the indices of the state's prefixes.
    ///
    /// # 戻り値
    ///
    /// The range of [`CarriedNbest::prefixes`] that holds them.
    fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }
}

/// A path of the current sentence pulled from an N-best search.
#[derive(Debug)]
struct Pulled {
    /// The path's tokens, with offsets relative to the sentence start.
    tokens: Vec<TokenOffset>,
    /// The state (BOS index) the path starts from.
    bos: usize,
    /// With `unique`, the class of the path's word boundaries within the
    /// sentence; 0 otherwise.
    class: u32,
    /// The path's tokens in [`CarriedNbest::tokens`], once a prefix uses
    /// them.
    stored: Option<(u32, u32)>,
}

/// The duplicate check of `unique`: a combination of a prefix and a path
/// repeats an earlier one when both the prefix's class and the path's class
/// do, i.e. when the word boundaries from the segment start are equal.
struct UniqueCheck<'s> {
    /// The classes of the paths of the current sentence, by their word
    /// boundaries.
    path_classes: &'s mut HashMap<Vec<(usize, usize)>, u32>,
    /// The (prefix class, path class) pairs already kept.
    seen: &'s mut HashSet<(u32, u32)>,
    /// The states of the previous sentence.
    states: &'s [State],
    /// The prefixes, for their classes.
    prefixes: &'s [Prefix],
}

/// The paths of one N-best search over the current sentence, as
/// [`merge_carried`] pulls them.
struct Pulls<'l, 's> {
    /// The search: the paths of one exit, or the paths through EOS.
    generator: NBestGenerator<'l>,
    /// The paths pulled so far, indexed as `merge_carried` counts them.
    pulled: &'s mut Vec<Pulled>,
    /// The duplicate check, with `unique` only.
    unique: Option<UniqueCheck<'s>>,
}

impl CarriedPaths for Pulls<'_, '_> {
    /// Pulls the next path of the search and, with `unique`, classifies its
    /// word boundaries.
    ///
    /// # 戻り値
    ///
    /// The path's cost, which includes the cost of its BOS edge (its state's
    /// best prefix, relative to the cheapest state), and its BOS index.
    fn next_path(&mut self) -> Option<(i64, usize)> {
        let ((tokens, cost), bos) = self.generator.next_with_bos()?;
        let class = match &mut self.unique {
            Some(check) => {
                let key: Vec<(usize, usize)> =
                    tokens.iter().map(|&(start, end, _)| (start, end)).collect();
                let next_class = check.path_classes.len() as u32;
                *check.path_classes.entry(key).or_insert(next_class)
            }
            None => 0,
        };
        self.pulled.push(Pulled {
            tokens,
            bos,
            class,
            stored: None,
        });
        Some((cost, bos))
    }

    /// Checks the word boundaries of the combination against the kept ones
    /// with `unique`; every combination is new without it.
    ///
    /// # 引数
    ///
    /// * `path` - The index of the pulled path.
    /// * `rank` - The index of the prefix in the path's state.
    ///
    /// # 戻り値
    ///
    /// Whether the combination is kept.
    fn is_new(&mut self, path: usize, rank: usize) -> bool {
        let Some(check) = &mut self.unique else {
            return true;
        };
        let pulled = &self.pulled[path];
        let state = &check.states[pulled.bos];
        let prefix_class = check.prefixes[state.start as usize + rank].class;
        check.seen.insert((prefix_class, pulled.class))
    }
}

/// The N-best search of one segment whose sentences carry the context, with
/// its buffers. See the module documentation.
pub(super) struct CarriedNbest {
    /// The maximum number of results, and of prefixes per state.
    n: usize,
    /// Whether to drop results whose word boundaries repeat a cheaper
    /// result's.
    unique: bool,
    /// The maximum cost above the best, per state and per segment.
    cost_threshold: Option<i64>,
    /// The start of the segment in the input, in bytes.
    segment_start: usize,
    /// The cost of every prefix from the segment start, without the EOS
    /// connection; indexed like `prefixes`.
    costs: Vec<i64>,
    /// Every prefix of the segment so far, the root first.
    prefixes: Vec<Prefix>,
    /// The tokens of the prefixes' parts.
    tokens: Vec<TokenOffset>,
    /// The states of the last sentence, in descending order of their
    /// exits' edge indices (the order of their BOS edges in the next
    /// sentence).
    states: Vec<State>,
    /// The cost of the cheapest state, which the costs of the current
    /// lattice are relative to.
    base: i64,
    /// The BOS contexts of the current sentence, one per state.
    contexts: Vec<BosContext>,
    /// Scratch: the exits of the current sentence.
    exits: Vec<LatticeExit>,
    /// Scratch: the states of the current sentence.
    next_states: Vec<State>,
    /// Scratch: the prefixes of the current sentence, before they join
    /// `prefixes`.
    new_prefixes: Vec<Prefix>,
    /// Scratch: the costs of `new_prefixes`.
    new_costs: Vec<i64>,
    /// Scratch: the paths pulled for one exit.
    pulled: Vec<Pulled>,
    /// Scratch of `unique`: the classes of the current sentence's paths.
    path_classes: HashMap<Vec<(usize, usize)>, u32>,
    /// Scratch of `unique`: the classes of the current sentence's prefixes,
    /// by the class of the prefix they extend and of their part.
    prefix_classes: HashMap<(u32, u32), u32>,
    /// Scratch of `unique`: the combinations kept for one exit.
    seen: HashSet<(u32, u32)>,
    /// Scratch: the exits of the current sentence by ascending cost.
    order: Vec<usize>,
    /// Scratch: the right id and the `n`-th prefix's cost (relative to
    /// `base`) of every state of the current sentence that has `n`
    /// prefixes.
    full: Vec<(u16, i64)>,
}

impl CarriedNbest {
    /// Creates the search with empty buffers.
    ///
    /// # 引数
    ///
    /// * `n` - The maximum number of results; at least 1.
    /// * `unique` - Whether to drop repeated word boundaries.
    /// * `cost_threshold` - If `Some(t)`, the maximum cost above the best;
    ///   not negative.
    ///
    /// # 戻り値
    ///
    /// The search.
    pub(super) fn new(n: usize, unique: bool, cost_threshold: Option<i64>) -> Self {
        Self {
            n,
            unique,
            cost_threshold,
            segment_start: 0,
            costs: Vec::new(),
            prefixes: Vec::new(),
            tokens: Vec::new(),
            states: Vec::new(),
            base: 0,
            contexts: Vec::new(),
            exits: Vec::new(),
            next_states: Vec::new(),
            new_prefixes: Vec::new(),
            new_costs: Vec::new(),
            pulled: Vec::new(),
            path_classes: HashMap::new(),
            prefix_classes: HashMap::new(),
            seen: HashSet::new(),
            order: Vec::new(),
            full: Vec::new(),
        }
    }

    /// Starts a segment: one state, the dictionary's BOS context, with the
    /// empty prefix.
    ///
    /// # 引数
    ///
    /// * `segment_start` - The start of the segment in the input, in bytes.
    pub(super) fn start(&mut self, segment_start: usize) {
        self.segment_start = segment_start;
        self.costs.clear();
        self.prefixes.clear();
        self.tokens.clear();
        self.states.clear();
        self.base = 0;
        self.costs.push(0);
        self.prefixes.push(Prefix {
            parent: NO_PREFIX,
            base: segment_start,
            tokens: (0, 0),
            class: 0,
        });
        self.states.push(State {
            right_id: 0,
            start: 0,
            len: 1,
            rank: 0,
        });
    }

    /// Returns the start of the segment.
    ///
    /// # 戻り値
    ///
    /// The start in the input, in bytes.
    pub(super) fn segment_start(&self) -> usize {
        self.segment_start
    }

    /// Prepares the BOS contexts of the next sentence of the segment: one
    /// per state, with its best prefix's cost relative to the cheapest
    /// state's.
    ///
    /// # 戻り値
    ///
    /// The contexts, for
    /// [`LatticeOptions::bos`](lindera_dictionary::viterbi::LatticeOptions::bos).
    pub(super) fn bos_contexts(&mut self) -> &[BosContext] {
        let Self {
            costs,
            states,
            contexts,
            ..
        } = self;
        self.base = fill_bos_contexts(
            states
                .iter()
                .map(|state| (state.right_id, costs[state.start as usize])),
            contexts,
        );
        &self.contexts
    }

    /// Moves to the end of a sentence whose cut carries the context: every
    /// exit of the sentence that another state does not rule out (see the
    /// module documentation) becomes a state, whose prefixes are the `n`
    /// cheapest combinations of a path to the exit with a prefix of the
    /// state the path starts from.
    ///
    /// # 引数
    ///
    /// * `lattice` - The sentence's N-best lattice, built from the BOS
    ///   contexts of [`CarriedNbest::bos_contexts`], or from the default BOS
    ///   edge for the first sentence of the segment.
    /// * `sentence_start` - The start of the sentence in the input.
    /// * `margins` - The segmenter's cache of exit margins.
    /// * `matrix` - The connection cost matrix of the segmenter's
    ///   dictionary.
    ///
    /// # 戻り値
    ///
    /// `false` when the sentence has no exit; the states are then left
    /// unchanged.
    pub(super) fn carry(
        &mut self,
        lattice: &Lattice,
        sentence_start: usize,
        margins: &MarginCache,
        matrix: &ConnectionCostMatrix,
    ) -> bool {
        let Self {
            n,
            unique,
            cost_threshold,
            costs,
            prefixes,
            tokens,
            states,
            base,
            exits,
            next_states,
            new_prefixes,
            new_costs,
            pulled,
            path_classes,
            prefix_classes,
            seen,
            order,
            full,
            ..
        } = self;
        lattice.exits_into(exits);
        path_classes.clear();
        prefix_classes.clear();
        next_states.clear();
        new_prefixes.clear();
        new_costs.clear();
        let offset = prefixes.len();
        let groups: Vec<&[i64]> = states.iter().map(|state| &costs[state.range()]).collect();

        // The cheapest exits first, so that the states that can rule out
        // the others are known before them.
        order.clear();
        order.extend(0..exits.len());
        order.sort_by_key(|&index| exits[index].cost());
        full.clear();
        for &index in order.iter() {
            let exit = &exits[index];
            let beaten = full.iter().any(|&(right_id, nth)| {
                i64::from(exit.cost()) - nth
                    > i64::from(margins.margin(matrix, exit.right_id(), right_id))
            });
            if beaten {
                continue;
            }
            pulled.clear();
            seen.clear();
            let mut paths = Pulls {
                generator: NBestGenerator::from_exit(lattice, exit),
                pulled: &mut *pulled,
                unique: unique.then(|| UniqueCheck {
                    path_classes: &mut *path_classes,
                    seen: &mut *seen,
                    states,
                    prefixes,
                }),
            };
            let picks = merge_carried(&mut paths, &groups, *n, *cost_threshold);
            if picks.is_empty() {
                continue;
            }
            let start = (offset + new_prefixes.len()) as u32;
            for pick in &picks {
                let path = &mut pulled[pick.path];
                let range = *path.stored.get_or_insert_with(|| {
                    let range_start = tokens.len() as u32;
                    tokens.extend_from_slice(&path.tokens);
                    (range_start, tokens.len() as u32)
                });
                let parent = states[path.bos].start + pick.rank as u32;
                let class = if *unique {
                    let key = (prefixes[parent as usize].class, path.class);
                    let next_class = prefix_classes.len() as u32;
                    *prefix_classes.entry(key).or_insert(next_class)
                } else {
                    0
                };
                new_prefixes.push(Prefix {
                    parent,
                    base: sentence_start,
                    tokens: range,
                    class,
                });
                new_costs.push(base.saturating_add(pick.cost));
            }
            if picks.len() == *n
                && let Some(last) = picks.last()
            {
                full.push((exit.right_id(), last.cost));
            }
            next_states.push(State {
                right_id: exit.right_id(),
                start,
                len: picks.len() as u32,
                rank: exit.edge_index(),
            });
        }

        if next_states.is_empty() {
            return false;
        }
        prefixes.append(new_prefixes);
        costs.append(new_costs);
        std::mem::swap(states, next_states);
        // The BOS edges of the next sentence follow the order of the 1-best
        // segmentation: descending edge index, so that the first one, which
        // wins ties, is the exit one lattice over the line would keep.
        states.sort_unstable_by_key(|state| Reverse(state.rank));
        true
    }

    /// Ends the segment with its last sentence: the results are the `n`
    /// cheapest combinations of a path through EOS with a prefix of the
    /// state the path starts from, with the threshold measured from the
    /// segment's best and `unique` over the word boundaries of the whole
    /// segment.
    ///
    /// # 引数
    ///
    /// * `lattice` - The last sentence's N-best lattice, built from the BOS
    ///   contexts of [`CarriedNbest::bos_contexts`].
    /// * `sentence_start` - The start of the sentence in the input.
    ///
    /// # 戻り値
    ///
    /// The results in ascending order of cost, each its tokens (offsets
    /// relative to the segment start) and its cost; empty when the sentence
    /// has no complete path.
    pub(super) fn finish(&mut self, lattice: &Lattice, sentence_start: usize) -> Vec<NBestPath> {
        self.pulled.clear();
        self.seen.clear();
        self.path_classes.clear();
        let groups: Vec<&[i64]> = self
            .states
            .iter()
            .map(|state| &self.costs[state.range()])
            .collect();
        let mut paths = Pulls {
            generator: NBestGenerator::new(lattice),
            pulled: &mut self.pulled,
            unique: self.unique.then(|| UniqueCheck {
                path_classes: &mut self.path_classes,
                seen: &mut self.seen,
                states: &self.states,
                prefixes: &self.prefixes,
            }),
        };
        let picks = merge_carried(&mut paths, &groups, self.n, self.cost_threshold);

        let shift = sentence_start - self.segment_start;
        picks
            .iter()
            .map(|pick| {
                let path = &self.pulled[pick.path];
                let parent = self.states[path.bos].start + pick.rank as u32;
                let mut offsets = self.prefix_tokens(parent);
                offsets.extend(
                    path.tokens
                        .iter()
                        .map(|&(start, end, word_id)| (start + shift, end + shift, word_id)),
                );
                (offsets, self.base.saturating_add(pick.cost))
            })
            .collect()
    }

    /// Ends the segment early, at the end of the last sentence that carried
    /// the context: for a sentence that has no exit or no complete path,
    /// which a dictionary that covers every character with an unknown word
    /// never gives. The segment ends there, so every prefix pays the EOS
    /// connection from its state's right id, and the results are the `n`
    /// cheapest prefixes over all states with it. On equal cost the state
    /// that comes first wins, then the prefix that comes first in its
    /// state, so the first result is the path the 1-best segmentation
    /// commits (#1133).
    ///
    /// # Arguments
    ///
    /// * `matrix` - The connection cost matrix of the segmenter's
    ///   dictionary, for the EOS connection.
    ///
    /// # Returns
    ///
    /// The results in ascending order of cost, each its tokens (offsets
    /// relative to the segment start) and its cost, the EOS connection
    /// included.
    pub(super) fn commit(&self, matrix: &ConnectionCostMatrix) -> Vec<NBestPath> {
        let mut all: Vec<(i64, usize, u32)> = self
            .states
            .iter()
            .enumerate()
            .flat_map(|(position, state)| {
                let eos = i64::from(matrix.cost(u32::from(state.right_id), 0));
                state.range().map(move |index| {
                    (
                        self.costs[index].saturating_add(eos),
                        position,
                        index as u32,
                    )
                })
            })
            .collect();
        all.sort_unstable();
        let Some(&(best, _, _)) = all.first() else {
            return Vec::new();
        };
        let mut seen = HashSet::new();
        all.into_iter()
            .take_while(|&(cost, _, _)| {
                self.cost_threshold
                    .is_none_or(|threshold| cost.saturating_sub(best) <= threshold)
            })
            .filter(|&(_, _, index)| {
                !self.unique || seen.insert(self.prefixes[index as usize].class)
            })
            .take(self.n)
            .map(|(cost, _, index)| (self.prefix_tokens(index), cost))
            .collect()
    }

    /// Returns the tokens of a prefix, part by part from the segment start.
    ///
    /// # 引数
    ///
    /// * `prefix` - The index of the prefix.
    ///
    /// # 戻り値
    ///
    /// The tokens, with offsets relative to the segment start.
    fn prefix_tokens(&self, prefix: u32) -> Vec<TokenOffset> {
        let mut chain = Vec::new();
        let mut current = prefix;
        while current != NO_PREFIX {
            chain.push(current);
            current = self.prefixes[current as usize].parent;
        }
        let mut offsets = Vec::new();
        for &index in chain.iter().rev() {
            let part = &self.prefixes[index as usize];
            let shift = part.base - self.segment_start;
            let (start, end) = part.tokens;
            offsets.extend(self.tokens[start as usize..end as usize].iter().map(
                |&(byte_start, byte_end, word_id)| (byte_start + shift, byte_end + shift, word_id),
            ));
        }
        offsets
    }
}

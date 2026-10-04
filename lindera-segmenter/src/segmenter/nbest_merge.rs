//! Combination of N-best lists into whole-input N-best results.
//!
//! The segmenter runs the N-best search segment by segment, a segment being
//! a run of sentences that carry the context from one to the next (ended by
//! `\n`, `\t` or the end of the input). Segment boundaries are fixed and no
//! connection cost spans them, so the cost of a segmentation of the whole
//! input is the sum of the costs of the paths it takes in each segment.
//! [`merge_nbest`] picks the `n` cheapest of these combinations.
//!
//! [`merge_carried`] is the step for a sentence that continues the context
//! of the previous one inside a segment: the sentence's paths start from
//! several BOS edges, one per group of earlier results (a state), and each
//! path combines with the results of its own group only.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A combination kept after one merge step: a choice of path for each of the
/// segments merged so far.
#[derive(Clone, Copy, Debug)]
struct Node {
    /// The total cost of the paths chosen so far.
    cost: i64,
    /// The index of the combination this one extends, in the previous step's
    /// list.
    parent: usize,
    /// The rank of the path chosen in the segment of this step.
    rank: usize,
}

/// Combines per-segment N-best cost lists into the `n` cheapest
/// combinations, choosing one path per segment.
///
/// Every list must be sorted in ascending order, as the N-best search
/// returns it. The combinations come out in ascending order of total cost,
/// and ties are broken the same way on every call. The first combination
/// takes the first path of every segment.
///
/// The lists are merged from left to right. Each step merges the running
/// list of combinations with the next segment's list through a min-heap
/// keyed by `(cost, i, j)`. A pair `(i, j)` is pushed only by its single
/// parent, `(i, j - 1)` or, when `j == 0`, `(i - 1, 0)`, and keys strictly
/// increase from parent to child. So the pops are sorted by the key and no
/// pair is visited twice. Keeping only `n` combinations per step is exact:
/// a combination whose prefix is not among the `n` best prefixes has `n`
/// strictly better combinations that share its suffix.
///
/// # 引数
///
/// * `lists` - The ascending path costs of each segment, in input order.
/// * `n` - The maximum number of combinations to return.
/// * `cost_threshold` - If `Some(t)`, drop the combinations whose total cost
///   exceeds the cost of the first combination by more than `t`.
///
/// # 戻り値
///
/// The combinations, each as the rank chosen in every segment and the total
/// cost. Empty if `n` is zero, `lists` is empty, any list is empty, or
/// `cost_threshold` is negative.
pub(super) fn merge_nbest<L: AsRef<[i64]>>(
    lists: &[L],
    n: usize,
    cost_threshold: Option<i64>,
) -> Vec<(Vec<usize>, i64)> {
    if n == 0 || lists.is_empty() || lists.iter().any(|list| list.as_ref().is_empty()) {
        return Vec::new();
    }

    // The empty combination, which the first step extends.
    let root = [Node {
        cost: 0,
        parent: 0,
        rank: 0,
    }];
    let mut steps: Vec<Vec<Node>> = Vec::with_capacity(lists.len());
    // The cost of the first combination of the segments merged so far.
    let mut best_cost: i64 = 0;

    for list in lists {
        let list = list.as_ref();
        debug_assert!(list.is_sorted(), "N-best costs must be ascending");

        best_cost = best_cost.saturating_add(list[0]);
        let previous = steps.last().map_or(&root[..], Vec::as_slice);
        let step = merge_step(previous, list, n, best_cost, cost_threshold);
        if step.is_empty() {
            return Vec::new();
        }
        steps.push(step);
    }

    let Some(last) = steps.last() else {
        return Vec::new();
    };
    (0..last.len())
        .map(|index| {
            let mut ranks = vec![0; steps.len()];
            let mut node_index = index;
            for (segment, step) in steps.iter().enumerate().rev() {
                let node = step[node_index];
                ranks[segment] = node.rank;
                node_index = node.parent;
            }
            (ranks, last[index].cost)
        })
        .collect()
}

/// Merges the combinations of the previous segments with the paths of the
/// next segment, keeping the `n` cheapest.
///
/// # 引数
///
/// * `previous` - The combinations of the previous segments, ascending.
/// * `list` - The ascending path costs of the next segment; not empty.
/// * `n` - The maximum number of combinations to keep.
/// * `best_cost` - The cost of the first combination including this segment.
/// * `cost_threshold` - If `Some(t)`, stop at the first combination whose
///   cost exceeds `best_cost` by more than `t`. The excess only grows from
///   one pop to the next, and the remaining segments cannot lower it.
///
/// # 戻り値
///
/// The combinations including this segment, ascending.
fn merge_step(
    previous: &[Node],
    list: &[i64],
    n: usize,
    best_cost: i64,
    cost_threshold: Option<i64>,
) -> Vec<Node> {
    let mut merged = Vec::with_capacity(n.min(previous.len().saturating_mul(list.len())));
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((previous[0].cost.saturating_add(list[0]), 0, 0)));

    while merged.len() < n {
        let Some(Reverse((cost, i, j))) = heap.pop() else {
            break;
        };
        if let Some(threshold) = cost_threshold
            && cost.saturating_sub(best_cost) > threshold
        {
            break;
        }
        merged.push(Node {
            cost,
            parent: i,
            rank: j,
        });

        if j + 1 < list.len() {
            heap.push(Reverse((
                previous[i].cost.saturating_add(list[j + 1]),
                i,
                j + 1,
            )));
        }
        if j == 0 && i + 1 < previous.len() {
            heap.push(Reverse((
                previous[i + 1].cost.saturating_add(list[0]),
                i + 1,
                0,
            )));
        }
    }

    merged
}

/// The paths of one sentence that [`merge_carried`] combines with the
/// groups of earlier results, pulled lazily from an N-best search.
pub(super) trait CarriedPaths {
    /// Pulls the next path, in ascending order of cost.
    ///
    /// # 戻り値
    ///
    /// The path's cost combined with the first (cheapest) result of its
    /// group, and its group: the index of the state, i.e. of the BOS edge,
    /// that it starts from. `None` when there are no more paths. Path `i`
    /// is the one returned by the `i`-th call, counted from 0.
    fn next_path(&mut self) -> Option<(i64, usize)>;

    /// Returns whether combining a path with a result of its group gives a
    /// new result, i.e. one that no result merged before it repeats; this is
    /// how `unique` drops repeated word boundaries. Called in the order of
    /// the merge, at most once per combination.
    ///
    /// # 引数
    ///
    /// * `path` - The index of the path (see
    ///   [`CarriedPaths::next_path`]).
    /// * `rank` - The index of the result in the path's group.
    ///
    /// # 戻り値
    ///
    /// `true` to keep the combination, `false` to drop it as a duplicate.
    fn is_new(&mut self, path: usize, rank: usize) -> bool;
}

/// One combination that [`merge_carried`] keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CarriedPick {
    /// The index of the path (see [`CarriedPaths::next_path`]).
    pub(super) path: usize,
    /// The index of the result of the path's group that it extends.
    pub(super) rank: usize,
    /// The cost of the combination: the path's cost plus how much the
    /// result costs above the first result of its group.
    pub(super) cost: i64,
}

/// Combines the paths of a sentence with the earlier results they continue,
/// keeping the `n` cheapest combinations.
///
/// Every path starts from one state (a BOS edge), whose results are its
/// group in `groups`, ascending. The paths come in ascending order of their
/// cost combined with the first result of their group. Combining path `i`
/// with result `k` of its group `g` costs
/// `cost_i + groups[g][k] - groups[g][0]`.
///
/// The combinations come out in ascending order of the key `(cost, i, k)`.
/// A min-heap holds the frontier: a pair `(i, k)` is pushed only by its
/// single parent, `(i, k - 1)` or, when `k == 0`, `(i - 1, 0)`, and the key
/// strictly increases from parent to child, so the pops are sorted by the
/// key and no pair is visited twice. Path `i + 1` is pulled only when
/// `(i, 0)` is popped, so the search runs no further than the combinations
/// need.
///
/// # 引数
///
/// * `paths` - The paths, pulled lazily, and the duplicate check.
/// * `groups` - The ascending costs of the results of every state, indexed
///   by the group a path returns. A path whose group is empty combines with
///   nothing.
/// * `n` - The maximum number of combinations to keep.
/// * `cost_threshold` - If `Some(t)`, stop at the first combination whose
///   cost exceeds the first combination's by more than `t`.
///
/// # 戻り値
///
/// The kept combinations, ascending: duplicates (see
/// [`CarriedPaths::is_new`]) are skipped and do not count toward `n`. Empty
/// if `n` is zero, there is no path, or `cost_threshold` is negative.
pub(super) fn merge_carried<L: AsRef<[i64]>>(
    paths: &mut impl CarriedPaths,
    groups: &[L],
    n: usize,
    cost_threshold: Option<i64>,
) -> Vec<CarriedPick> {
    let mut picks = Vec::new();
    if n == 0 {
        return picks;
    }
    // The pulled paths: cost and group, indexed by path.
    let mut pulled: Vec<(i64, usize)> = Vec::new();
    let Some(first) = paths.next_path() else {
        return picks;
    };
    pulled.push(first);
    let mut heap = BinaryHeap::new();
    heap.push(Reverse((first.0, 0_usize, 0_usize)));
    // The cost of the first combination.
    let mut best_cost: Option<i64> = None;

    while picks.len() < n {
        let Some(Reverse((cost, i, k))) = heap.pop() else {
            break;
        };
        let (path_cost, group) = pulled[i];
        let group = groups.get(group).map_or(&[][..], AsRef::as_ref);

        if k + 1 < group.len() {
            let extra = group[k + 1].saturating_sub(group[0]);
            heap.push(Reverse((path_cost.saturating_add(extra), i, k + 1)));
        }
        if k == 0 {
            // `(i, 0)` is popped once, right after path `i` is the last one
            // pulled, so the next path is `i + 1`.
            if let Some(next) = paths.next_path() {
                debug_assert!(next.0 >= path_cost, "paths must come in ascending order");
                pulled.push(next);
                heap.push(Reverse((next.0, i + 1, 0)));
            }
        }
        if group.is_empty() {
            continue;
        }

        let best = *best_cost.get_or_insert(cost);
        if let Some(threshold) = cost_threshold
            && cost.saturating_sub(best) > threshold
        {
            break;
        }
        if paths.is_new(i, k) {
            picks.push(CarriedPick {
                path: i,
                rank: k,
                cost,
            });
        }
    }

    picks
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{CarriedPaths, CarriedPick, merge_carried, merge_nbest};

    /// All ascending lists of length 1 to 3 over `{-1, 0, 1}`.
    fn ascending_lists() -> Vec<Vec<i64>> {
        let values = [-1_i64, 0, 1];
        let mut lists = Vec::new();
        for &a in &values {
            lists.push(vec![a]);
            for &b in values.iter().filter(|&&b| b >= a) {
                lists.push(vec![a, b]);
                for &c in values.iter().filter(|&&c| c >= b) {
                    lists.push(vec![a, b, c]);
                }
            }
        }
        lists
    }

    /// Every combination of one rank per list, in odometer order.
    fn product(lists: &[Vec<i64>]) -> Vec<Vec<usize>> {
        let mut combinations = vec![Vec::new()];
        for list in lists {
            combinations = combinations
                .into_iter()
                .flat_map(|prefix| {
                    (0..list.len()).map(move |rank| {
                        let mut combination = prefix.clone();
                        combination.push(rank);
                        combination
                    })
                })
                .collect();
        }
        combinations
    }

    /// The expected result: the product sorted by the merge order
    /// `(C_m, ..., C_1, j_1, ..., j_m)`, where `C_k` is the cost of the
    /// first `k` sentences and `j_k` the rank in sentence `k`, filtered by
    /// the threshold and cut to `n`.
    fn oracle(lists: &[Vec<i64>], n: usize, threshold: Option<i64>) -> Vec<(Vec<usize>, i64)> {
        let best: i64 = lists.iter().map(|list| list[0]).sum();
        let mut keyed: Vec<(Vec<i64>, Vec<usize>, i64)> = product(lists)
            .into_iter()
            .map(|ranks| {
                let mut prefix_costs = Vec::new();
                let mut total = 0;
                for (list, &rank) in lists.iter().zip(&ranks) {
                    total += list[rank];
                    prefix_costs.push(total);
                }
                let mut key: Vec<i64> = prefix_costs.into_iter().rev().collect();
                key.extend(ranks.iter().map(|&rank| rank as i64));
                (key, ranks, total)
            })
            .filter(|(_, _, total)| threshold.is_none_or(|t| total - best <= t))
            .collect();
        keyed.sort();
        keyed
            .into_iter()
            .take(n)
            .map(|(_, ranks, total)| (ranks, total))
            .collect()
    }

    #[test]
    fn test_merge_nbest_matches_brute_force() {
        let lists = ascending_lists();
        let ns = [1, 2, 3, 5, 30];
        let thresholds = [None, Some(0), Some(1), Some(2), Some(3)];

        // Every list alone and every pair; triples use the lists of length
        // 1 or 2 to keep the run short.
        let short: Vec<&Vec<i64>> = lists.iter().filter(|list| list.len() <= 2).collect();
        let mut cases: Vec<Vec<Vec<i64>>> = lists.iter().map(|a| vec![a.clone()]).collect();
        for a in &lists {
            for b in &lists {
                cases.push(vec![a.clone(), b.clone()]);
            }
        }
        for &a in &short {
            for &b in &short {
                for &c in &short {
                    cases.push(vec![a.clone(), b.clone(), c.clone()]);
                }
            }
        }

        for case in &cases {
            for &n in &ns {
                for &threshold in &thresholds {
                    assert_eq!(
                        merge_nbest(case, n, threshold),
                        oracle(case, n, threshold),
                        "lists {case:?}, n {n}, threshold {threshold:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_merge_nbest_global_order_differs_from_rank_join() {
        // `東京、です` with IPADIC, cut at `、` into two independent lists as
        // before the context was carried: joining rank r of every list gives
        // 3079 / 23850 / 24249, the global top 3 keeps the first list.
        let lists = [vec![-1883, 10284, 10351], vec![4962, 13566, 13898]];
        assert_eq!(
            merge_nbest(&lists, 3, None),
            vec![(vec![0, 0], 3079), (vec![0, 1], 11683), (vec![0, 2], 12015)]
        );
    }

    #[test]
    fn test_merge_nbest_short_list_stays_in_every_rank() {
        // `。東京` with IPADIC, cut after `。` into two independent lists as
        // before the context was carried: `。` has two paths; every
        // combination still takes one of them.
        let lists = [vec![-1910, 16980], vec![1923, 13453, 13520]];
        assert_eq!(
            merge_nbest(&lists, 3, None),
            vec![(vec![0, 0], 13), (vec![0, 1], 11543), (vec![0, 2], 11610)]
        );
    }

    #[test]
    fn test_merge_nbest_threshold_is_relative_to_the_whole_input() {
        // Each sentence's second path is within 5 of its best, but taking
        // both is 8 above the best combination. On equal cost the later
        // sentence changes first.
        let lists = [vec![0, 4], vec![0, 4]];
        assert_eq!(
            merge_nbest(&lists, 10, Some(5)),
            vec![(vec![0, 0], 0), (vec![0, 1], 4), (vec![1, 0], 4)]
        );
        assert_eq!(merge_nbest(&lists, 10, Some(8)).len(), 4);
    }

    #[test]
    fn test_merge_nbest_empty_inputs() {
        let no_lists: [Vec<i64>; 0] = [];
        assert!(merge_nbest(&no_lists, 3, None).is_empty());
        assert!(merge_nbest(&[vec![1, 2]], 0, None).is_empty());
        assert!(merge_nbest(&[vec![1, 2], vec![]], 3, None).is_empty());
    }

    #[test]
    fn test_merge_nbest_product_smaller_than_n() {
        let lists = [vec![0, 1], vec![0]];
        assert_eq!(
            merge_nbest(&lists, 5, None),
            vec![(vec![0, 0], 0), (vec![1, 0], 1)]
        );
    }

    #[test]
    fn test_merge_nbest_single_list_is_identity() {
        let list = vec![-3, 2, 2, 7];
        let expected: Vec<(Vec<usize>, i64)> = list
            .iter()
            .enumerate()
            .map(|(rank, &cost)| (vec![rank], cost))
            .collect();
        let lists = std::slice::from_ref(&list);
        assert_eq!(merge_nbest(lists, 10, None), expected);
        assert_eq!(merge_nbest(lists, 2, None), expected[..2].to_vec());
    }

    #[test]
    fn test_merge_nbest_many_sentences() {
        let lists = vec![vec![0_i64, 1]; 2000];
        let results = merge_nbest(&lists, 3, None);
        assert_eq!(results.len(), 3);

        let mut second = vec![0; 2000];
        second[1999] = 1;
        let mut third = vec![0; 2000];
        third[1998] = 1;
        assert_eq!(results[0], (vec![0; 2000], 0));
        assert_eq!(results[1], (second, 1));
        assert_eq!(results[2], (third, 1));

        assert_eq!(merge_nbest(&lists, 3, Some(0)).len(), 1);
    }

    #[test]
    fn test_merge_nbest_extreme_values() {
        let lists = [
            vec![-1_000_000_000_000, 1_000_000_000_000],
            vec![0, i64::MAX / 4],
        ];
        assert_eq!(merge_nbest(&lists, usize::MAX, Some(i64::MAX)).len(), 4);
        assert!(merge_nbest(&lists, 3, Some(-1)).is_empty());
    }

    /// A finite list of paths for `merge_carried`. A combination's word
    /// boundaries are stood in for by labels: the label of the result it
    /// extends and the label of the path; with `unique`, a combination whose
    /// two labels repeat an earlier combination's is a duplicate.
    struct Paths<'a> {
        /// Each path's cost (combined with the first result of its group)
        /// and group.
        paths: &'a [(i64, usize)],
        /// Each path's label.
        path_labels: &'a [u32],
        /// Each result's label, per group.
        result_labels: &'a [Vec<u32>],
        /// The label pairs seen so far; `None` without `unique`.
        seen: Option<HashSet<(u32, u32)>>,
        /// How many paths were pulled.
        pulled: usize,
        /// The combinations checked by `is_new`, in call order.
        checked: Vec<(usize, usize)>,
    }

    impl<'a> Paths<'a> {
        fn new(
            paths: &'a [(i64, usize)],
            path_labels: &'a [u32],
            result_labels: &'a [Vec<u32>],
            unique: bool,
        ) -> Self {
            Self {
                paths,
                path_labels,
                result_labels,
                seen: unique.then(HashSet::new),
                pulled: 0,
                checked: Vec::new(),
            }
        }

        fn labels(&self, path: usize, rank: usize) -> (u32, u32) {
            let group = self.paths[path].1;
            (
                self.result_labels[group][rank],
                self.path_labels[path % self.path_labels.len()],
            )
        }
    }

    impl CarriedPaths for Paths<'_> {
        fn next_path(&mut self) -> Option<(i64, usize)> {
            let path = self.paths.get(self.pulled).copied()?;
            self.pulled += 1;
            Some(path)
        }

        fn is_new(&mut self, path: usize, rank: usize) -> bool {
            assert!(
                path < self.pulled,
                "path {path} checked before it was pulled"
            );
            assert!(
                !self.checked.contains(&(path, rank)),
                "({path}, {rank}) checked twice"
            );
            self.checked.push((path, rank));
            let labels = self.labels(path, rank);
            match &mut self.seen {
                Some(seen) => seen.insert(labels),
                None => true,
            }
        }
    }

    /// The expected result: every combination of a path with a result of
    /// its group, sorted by `(cost, path, rank)`, cut at the threshold above
    /// the first, without duplicates, cut to `n`.
    fn carried_oracle(
        paths: &[(i64, usize)],
        groups: &[Vec<i64>],
        labels: &Paths,
        n: usize,
        threshold: Option<i64>,
        unique: bool,
    ) -> Vec<CarriedPick> {
        let mut all: Vec<(i64, usize, usize)> = Vec::new();
        for (path, &(cost, group)) in paths.iter().enumerate() {
            let results = groups.get(group).map_or(&[][..], Vec::as_slice);
            for (rank, result) in results.iter().enumerate() {
                all.push((cost + result - results[0], path, rank));
            }
        }
        all.sort();
        let Some(&(best, _, _)) = all.first() else {
            return Vec::new();
        };
        let mut seen = HashSet::new();
        all.into_iter()
            .take_while(|&(cost, _, _)| threshold.is_none_or(|t| cost - best <= t))
            .filter(|&(_, path, rank)| !unique || seen.insert(labels.labels(path, rank)))
            .take(n)
            .map(|(cost, path, rank)| CarriedPick { path, rank, cost })
            .collect()
    }

    /// All lists of up to 3 paths with ascending costs over `{0, 1}`, each
    /// starting from group 0 or 1.
    fn path_lists() -> Vec<Vec<(i64, usize)>> {
        let mut lists: Vec<Vec<(i64, usize)>> = vec![Vec::new()];
        let mut last: Vec<Vec<(i64, usize)>> = vec![Vec::new()];
        for _ in 0..3 {
            let mut next = Vec::new();
            for list in &last {
                let min_cost = list.last().map_or(0, |&(cost, _)| cost);
                for cost in min_cost..=1 {
                    for group in 0..2 {
                        let mut longer = list.clone();
                        longer.push((cost, group));
                        next.push(longer);
                    }
                }
            }
            lists.extend(next.iter().cloned());
            last = next;
        }
        lists
    }

    #[test]
    fn test_merge_carried_matches_brute_force() {
        // Group 0 takes every ascending list of up to 3 results, group 1 the
        // lists of up to 2; both may also be empty.
        let mut first_groups = vec![Vec::new()];
        first_groups.extend(ascending_lists());
        let second_groups: Vec<Vec<i64>> = first_groups
            .iter()
            .filter(|list| list.len() <= 2)
            .cloned()
            .collect();
        // Results of different groups share labels; the second scheme gives
        // every path the same label, so paths of equal boundaries repeat.
        let result_labels = vec![vec![0, 1, 2], vec![2, 1, 0]];
        let label_schemes: [&[u32]; 2] = [&[0, 1], &[0]];
        let ns = [1, 2, 3, 30];
        let thresholds = [None, Some(0), Some(1), Some(2)];

        let path_lists = path_lists();
        for first in &first_groups {
            for second in &second_groups {
                let groups = vec![first.clone(), second.clone()];
                for paths in &path_lists {
                    for path_labels in label_schemes {
                        for unique in [false, true] {
                            for &n in &ns {
                                for &threshold in &thresholds {
                                    let mut source =
                                        Paths::new(paths, path_labels, &result_labels, unique);
                                    let picks = merge_carried(&mut source, &groups, n, threshold);
                                    let expected = carried_oracle(
                                        paths, &groups, &source, n, threshold, unique,
                                    );
                                    assert_eq!(
                                        picks, expected,
                                        "groups {groups:?}, paths {paths:?}, labels \
                                         {path_labels:?}, unique {unique}, n {n}, \
                                         threshold {threshold:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// A path is pulled only when the combinations reach it.
    #[test]
    fn test_merge_carried_pulls_lazily() {
        let paths: Vec<(i64, usize)> = (0..1000).map(|cost| (cost, 0)).collect();
        let result_labels = vec![vec![0, 1, 2]];
        for (results, n, expected_pulls) in [
            // Popping path i's first combination pulls path i + 1.
            (vec![0], 3, 4),
            // Every result of the first path is cheaper than the next path.
            (vec![0, 0, 0], 3, 2),
            (vec![0, 0, 5], 3, 3),
        ] {
            let groups = vec![results];
            let mut source = Paths::new(&paths, &[0], &result_labels, false);
            let picks = merge_carried(&mut source, &groups, n, None);
            assert_eq!(picks.len(), n);
            assert_eq!(source.pulled, expected_pulls, "groups {groups:?}");
        }

        let groups = vec![vec![0]];
        let mut source = Paths::new(&paths, &[0], &result_labels, false);
        assert!(merge_carried(&mut source, &groups, 0, None).is_empty());
        assert_eq!(source.pulled, 0);
    }

    /// On equal cost, the earlier path comes first, then the lower rank.
    #[test]
    fn test_merge_carried_tie_order() {
        let paths = [(5, 0), (5, 1)];
        let groups = vec![vec![2, 2], vec![-7]];
        let result_labels = vec![vec![0, 1], vec![0]];
        let mut source = Paths::new(&paths, &[0], &result_labels, false);
        assert_eq!(
            merge_carried(&mut source, &groups, 5, None),
            vec![
                CarriedPick {
                    path: 0,
                    rank: 0,
                    cost: 5
                },
                CarriedPick {
                    path: 0,
                    rank: 1,
                    cost: 5
                },
                CarriedPick {
                    path: 1,
                    rank: 0,
                    cost: 5
                },
            ]
        );
    }

    /// A path of an empty or unknown group combines with nothing, and the
    /// threshold is measured from the first kept combination.
    #[test]
    fn test_merge_carried_skips_paths_without_results() {
        let paths = [(1, 1), (2, 2), (3, 0), (4, 0)];
        let groups = vec![vec![0], Vec::new()];
        let result_labels = vec![vec![0], Vec::new()];
        let mut source = Paths::new(&paths, &[0, 1], &result_labels, false);
        assert_eq!(
            merge_carried(&mut source, &groups, 5, Some(0)),
            vec![CarriedPick {
                path: 2,
                rank: 0,
                cost: 3
            }]
        );
    }

    /// Duplicates do not count toward `n` and do not stop the merge.
    #[test]
    fn test_merge_carried_unique_skips_duplicates() {
        // Paths 0 and 1 have the same label, and so do results 0 and 2.
        let paths = [(0, 0), (1, 0), (2, 0)];
        let groups = vec![vec![0, 10, 20]];
        let result_labels = vec![vec![0, 1, 0]];
        let mut source = Paths::new(&paths, &[7, 7, 8], &result_labels, true);
        let picks = merge_carried(&mut source, &groups, 3, None);
        let kept: Vec<(usize, usize)> = picks.iter().map(|p| (p.path, p.rank)).collect();
        assert_eq!(kept, vec![(0, 0), (2, 0), (0, 1)]);
        assert_eq!(
            source.checked,
            vec![(0, 0), (1, 0), (2, 0), (0, 1)],
            "is_new follows the merge order"
        );
    }

    #[test]
    fn test_merge_carried_empty_inputs() {
        let groups = vec![vec![0]];
        let result_labels = vec![vec![0]];
        let mut source = Paths::new(&[], &[0], &result_labels, false);
        assert!(merge_carried(&mut source, &groups, 3, None).is_empty());

        let paths = [(0, 0)];
        let mut source = Paths::new(&paths, &[0], &result_labels, false);
        assert!(merge_carried(&mut source, &groups, 3, Some(-1)).is_empty());
    }

    #[test]
    fn test_merge_carried_extreme_values() {
        let paths = [(i64::MAX / 2, 0), (i64::MAX / 2, 0)];
        let groups = vec![vec![i64::MIN / 2, i64::MAX / 2]];
        let result_labels = vec![vec![0, 1]];
        let mut source = Paths::new(&paths, &[0, 1], &result_labels, false);
        let picks = merge_carried(&mut source, &groups, usize::MAX, Some(i64::MAX));
        assert_eq!(picks.len(), 4);
        assert!(picks.windows(2).all(|pair| pair[0].cost <= pair[1].cost));
        assert_eq!(picks[3].cost, i64::MAX);
    }
}

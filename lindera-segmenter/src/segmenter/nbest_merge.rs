//! Combination of per-sentence N-best lists into whole-input N-best results.
//!
//! The segmenter runs the N-best search sentence by sentence. Sentence
//! boundaries are fixed and no connection cost spans them, so the cost of a
//! segmentation of the whole input is the sum of the costs of the paths it
//! takes in each sentence. [`merge_nbest`] picks the `n` cheapest of these
//! combinations.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A combination kept after one merge step: a choice of path for each of the
/// sentences merged so far.
#[derive(Clone, Copy, Debug)]
struct Node {
    /// The total cost of the paths chosen so far.
    cost: i64,
    /// The index of the combination this one extends, in the previous step's
    /// list.
    parent: usize,
    /// The rank of the path chosen in the sentence of this step.
    rank: usize,
}

/// Combines per-sentence N-best cost lists into the `n` cheapest
/// combinations, choosing one path per sentence.
///
/// Every list must be sorted in ascending order, as the N-best search
/// returns it. The combinations come out in ascending order of total cost,
/// and ties are broken the same way on every call. The first combination
/// takes the first path of every sentence.
///
/// The lists are merged from left to right. Each step merges the running
/// list of combinations with the next sentence's list through a min-heap
/// keyed by `(cost, i, j)`. A pair `(i, j)` is pushed only by its single
/// parent, `(i, j - 1)` or, when `j == 0`, `(i - 1, 0)`, and keys strictly
/// increase from parent to child. So the pops are sorted by the key and no
/// pair is visited twice. Keeping only `n` combinations per step is exact:
/// a combination whose prefix is not among the `n` best prefixes has `n`
/// strictly better combinations that share its suffix.
///
/// # 引数
///
/// * `lists` - The ascending path costs of each sentence, in sentence order.
/// * `n` - The maximum number of combinations to return.
/// * `cost_threshold` - If `Some(t)`, drop the combinations whose total cost
///   exceeds the cost of the first combination by more than `t`.
///
/// # 戻り値
///
/// The combinations, each as the rank chosen in every sentence and the total
/// cost. Empty if `n` is zero, `lists` is empty, or any list is empty.
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
    // The cost of the first combination of the sentences merged so far.
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
            for (sentence, step) in steps.iter().enumerate().rev() {
                let node = step[node_index];
                ranks[sentence] = node.rank;
                node_index = node.parent;
            }
            (ranks, last[index].cost)
        })
        .collect()
}

/// Merges the combinations of the previous sentences with the paths of the
/// next sentence, keeping the `n` cheapest.
///
/// # 引数
///
/// * `previous` - The combinations of the previous sentences, ascending.
/// * `list` - The ascending path costs of the next sentence; not empty.
/// * `n` - The maximum number of combinations to keep.
/// * `best_cost` - The cost of the first combination including this sentence.
/// * `cost_threshold` - If `Some(t)`, stop at the first combination whose
///   cost exceeds `best_cost` by more than `t`. The excess only grows from
///   one pop to the next, and the remaining sentences cannot lower it.
///
/// # 戻り値
///
/// The combinations including this sentence, ascending.
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

#[cfg(test)]
mod tests {
    use super::merge_nbest;

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
        // `東京、です` with IPADIC: joining rank r of every sentence gives
        // 3079 / 23850 / 24249, the global top 3 keeps the first sentence.
        let lists = [vec![-1883, 10284, 10351], vec![4962, 13566, 13898]];
        assert_eq!(
            merge_nbest(&lists, 3, None),
            vec![(vec![0, 0], 3079), (vec![0, 1], 11683), (vec![0, 2], 12015)]
        );
    }

    #[test]
    fn test_merge_nbest_short_list_stays_in_every_rank() {
        // `。東京` with IPADIC: `。` has two paths; every combination still
        // takes one of them.
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
}

//! The loop of the parser: shift-reduce over the bunsetsu of a sentence,
//! from left to right (Sassano 2004), as `parseShiftReduce` of CaboCha and
//! `_parseLinear` of J.DepP do.
//!
//! A stack holds the bunsetsu that have no head yet. For each bunsetsu
//! taken as the target, the top of the stack is the source: while the
//! decision is "the source depends on the target", the source gets its head
//! and the next one is looked at. A source that is refused stays on the
//! stack, and the target goes on top of it.
//!
//! Every head is to the right, no two arcs cross, and the last bunsetsu is
//! the root: whatever is on the stack when it comes depends on it, and no
//! decision is asked. The number of decisions is under twice the number of
//! bunsetsu.

/// What a decision answers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Decision {
    /// Whether the source depends on the target.
    pub(crate) attach: bool,
    /// The score of the arc, kept when the source is attached.
    pub(crate) score: f64,
}

/// The arcs of a sentence.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Arcs {
    /// The head of each bunsetsu; `None` for the last one, the root.
    pub(crate) heads: Vec<Option<usize>>,
    /// The score of the arc of each bunsetsu: the one of the decision that
    /// attached it, and 0 where none was asked.
    pub(crate) scores: Vec<f64>,
}

/// Parses a sentence.
///
/// # Arguments
///
/// * `count` - The number of bunsetsu.
/// * `decide` - Decides whether a source depends on a target, given their
///   indices and, for each bunsetsu, the bunsetsu attached to it so far.
///
/// # Returns
///
/// The arcs; none for no bunsetsu.
pub(crate) fn parse(
    count: usize,
    mut decide: impl FnMut(usize, usize, &[Vec<usize>]) -> Decision,
) -> Arcs {
    let mut heads = vec![None; count];
    let mut scores = vec![0.0; count];
    let mut children = vec![Vec::new(); count];
    let mut stack = Vec::new();
    for target in 0..count {
        let last = target + 1 == count;
        while let Some(&source) = stack.last() {
            let score = if last {
                0.0
            } else {
                let decision = decide(source, target, &children);
                if !decision.attach {
                    break;
                }
                decision.score
            };
            heads[source] = Some(target);
            scores[source] = score;
            children[target].push(source);
            stack.pop();
        }
        stack.push(target);
    }
    Arcs { heads, scores }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses with the heads of an answer deciding, and returns the arcs
    /// and the decisions asked, each as `(source, target, children of the
    /// source, children of the target)`.
    #[allow(clippy::type_complexity)]
    fn replay(answer: &[Option<usize>]) -> (Arcs, Vec<(usize, usize, Vec<usize>, Vec<usize>)>) {
        let mut asked = Vec::new();
        let arcs = parse(answer.len(), |source, target, children| {
            asked.push((
                source,
                target,
                children[source].clone(),
                children[target].clone(),
            ));
            Decision {
                attach: answer[source] == Some(target),
                score: 1.5,
            }
        });
        (arcs, asked)
    }

    #[test]
    fn decisions_are_asked_in_the_order_of_the_stack() {
        // 0 -> 1, 1 -> 3, 2 -> 3, 3 -> 4.
        let answer = [Some(1), Some(3), Some(3), Some(4), None];
        let (arcs, asked) = replay(&answer);
        assert_eq!(arcs.heads, answer);
        let pairs: Vec<(usize, usize)> = asked.iter().map(|one| (one.0, one.1)).collect();
        // 0 is attached to 1; 1 is refused by 2 and stays under it; 2 and
        // then 1 are attached to 3; 3 and the last bunsetsu are not asked.
        assert_eq!(pairs, [(0, 1), (1, 2), (2, 3), (1, 3)]);
    }

    #[test]
    fn a_decision_sees_the_bunsetsu_attached_so_far() {
        let answer = [Some(1), Some(3), Some(3), Some(4), None];
        let (_, asked) = replay(&answer);
        // (1, 2): 0 is attached to the source.
        assert_eq!(asked[1], (1, 2, vec![0], vec![]));
        // (1, 3): 0 is attached to the source, 2 to the target.
        assert_eq!(asked[3], (1, 3, vec![0], vec![2]));
    }

    #[test]
    fn the_score_is_the_one_of_the_decision_that_attached() {
        let answer = [Some(1), Some(3), Some(3), Some(4), None];
        let (arcs, _) = replay(&answer);
        // 3 depends on the last bunsetsu, which is not asked.
        assert_eq!(arcs.scores, [1.5, 1.5, 1.5, 0.0, 0.0]);
    }

    #[test]
    fn everything_left_depends_on_the_last_bunsetsu() {
        // Every decision refuses.
        let mut asked = 0;
        let arcs = parse(4, |_, _, _| {
            asked += 1;
            Decision {
                attach: false,
                score: -1.0,
            }
        });
        assert_eq!(arcs.heads, [Some(3), Some(3), Some(3), None]);
        assert_eq!(arcs.scores, [0.0; 4]);
        // (0, 1) and (1, 2): a refusal ends the look at a target.
        assert_eq!(asked, 2);
    }

    #[test]
    fn no_decision_is_asked_for_two_bunsetsu_or_fewer() {
        for count in 0..=2 {
            let arcs = parse(count, |_, _, _| panic!("a decision was asked"));
            let heads: Vec<Option<usize>> = (0..count)
                .map(|index| (index + 1 < count).then_some(count - 1))
                .collect();
            assert_eq!(arcs.heads, heads, "{count} bunsetsu");
            assert_eq!(arcs.scores, vec![0.0; count], "{count} bunsetsu");
        }
    }
}

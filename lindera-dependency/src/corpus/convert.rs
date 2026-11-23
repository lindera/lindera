//! From the word dependencies of UD to bunsetsu dependencies.
//!
//! UD Japanese-GSD was annotated by bunsetsu and converted to words by head
//! rules; this undoes the conversion.
//!
//! 1. A word with `BunsetuBILabel=B` starts a bunsetsu.
//! 2. In each bunsetsu, the word whose head is outside the bunsetsu gives
//!    the head of the bunsetsu: the bunsetsu of that head.
//! 3. The bunsetsu with the word whose head is the root is the root.
//! 4. Coordination is put back. UD attaches every later conjunct to the
//!    first one, with an arc to the left; the Kyoto corpus and CaboCha make
//!    each conjunct depend on the next, and the last one on the head of the
//!    whole.
//! 5. A sentence that is not the tree the parsers assume is dropped.

use std::ops::Range;

use super::DropCause;
use super::conllu::UdSentence;
use crate::sentence::{ArcType, Violation, tree_violations};

/// The relation of a conjunct to the first conjunct in UD.
const CONJ: &str = "conj";

/// A bunsetsu over the words of a UD sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdBunsetsu {
    /// The words of the bunsetsu, as a range of indices into the words of
    /// the sentence.
    pub words: Range<usize>,
    /// The index of the bunsetsu this one depends on, or `None` for the
    /// root.
    pub head: Option<usize>,
    /// The type of the arc to the head: [`ArcType::Parallel`] between
    /// conjuncts, [`ArcType::Dependency`] elsewhere.
    pub arc_type: ArcType,
    /// The content head of the bunsetsu, as an index into the words of the
    /// sentence: its one word with `BunsetuPositionType=SEM_HEAD` or
    /// `ROOT`. `None` when the bunsetsu has no such word or more than one.
    pub content_head: Option<usize>,
}

/// The bunsetsu dependencies of a UD sentence: a tree as the parsers assume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BunsetsuTree {
    /// The bunsetsu, from left to right.
    pub bunsetsu: Vec<UdBunsetsu>,
    /// The number of bunsetsu with more than one word whose head is outside
    /// the bunsetsu. The last such word gave the head.
    pub ambiguous_heads: usize,
}

/// Converts the word dependencies of a UD sentence to bunsetsu dependencies.
///
/// # Arguments
///
/// * `sentence` - The sentence.
///
/// # Returns
///
/// The tree, or the causes for which the sentence is dropped, each once:
/// [`DropCause::Bunsetsu`] when the first word does not start a bunsetsu or
/// a bunsetsu has no word whose head is outside it, and otherwise every way
/// the result is not a tree ([`DropCause::Root`],
/// [`DropCause::HeadToTheLeft`], [`DropCause::CrossingArcs`]).
pub fn to_bunsetsu(sentence: &UdSentence) -> Result<BunsetsuTree, Vec<DropCause>> {
    let words = &sentence.words;

    // Step 1: the bunsetsu, and the bunsetsu of each word.
    if !words.first().is_some_and(|word| word.starts_bunsetsu) {
        return Err(vec![DropCause::Bunsetsu]);
    }
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut bunsetsu_of = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        if word.starts_bunsetsu {
            if let Some(last) = ranges.last_mut() {
                last.end = index;
            }
            ranges.push(index..words.len());
        }
        bunsetsu_of.push(ranges.len() - 1);
    }
    let count = ranges.len();

    // Steps 2 and 3: the head of each bunsetsu, and the relation of the word
    // that gives it. `Some(None)` is the root.
    let mut heads: Vec<Option<Option<usize>>> = vec![None; count];
    let mut conjunct = vec![false; count];
    let mut ambiguous_heads = 0;
    for (index, word) in words.iter().enumerate() {
        let bunsetsu = bunsetsu_of[index];
        let head = match word.head {
            0 => None,
            head => Some(bunsetsu_of[head - 1]),
        };
        if head == Some(bunsetsu) {
            continue;
        }
        if heads[bunsetsu].is_some() {
            ambiguous_heads += 1;
        }
        heads[bunsetsu] = Some(head);
        conjunct[bunsetsu] = head.is_some() && word.deprel == CONJ;
    }
    let Some(heads) = heads.into_iter().collect::<Option<Vec<Option<usize>>>>() else {
        return Err(vec![DropCause::Bunsetsu]);
    };

    // Step 4.
    let (heads, arc_types) = restore_coordination(&heads, &conjunct);

    // Step 5.
    let causes = drop_causes(&heads);
    if !causes.is_empty() {
        return Err(causes);
    }

    let bunsetsu = ranges
        .into_iter()
        .zip(heads)
        .zip(arc_types)
        .map(|((range, head), arc_type)| {
            let mut content_heads = range.clone().filter(|&index| words[index].content_head);
            let content_head = match (content_heads.next(), content_heads.next()) {
                (Some(index), None) => Some(index),
                _ => None,
            };
            UdBunsetsu {
                words: range,
                head,
                arc_type,
                content_head,
            }
        })
        .collect();
    Ok(BunsetsuTree {
        bunsetsu,
        ambiguous_heads,
    })
}

/// Puts coordination back in the form of the Kyoto corpus.
///
/// For a bunsetsu `a` with the conjuncts `b` and `c`, in this order, and the
/// head `h`, the arcs `a → h`, `b → a`, `c → a` of UD become `a → b`,
/// `b → c`, `c → h`; a conjunct with conjuncts of its own is treated the
/// same way, before the next conjunct. The arcs between conjuncts get the
/// type [`ArcType::Parallel`].
///
/// # Arguments
///
/// * `heads` - The head of each bunsetsu, `None` for the root.
/// * `conjunct` - Whether each bunsetsu is a conjunct of its head.
///
/// # Returns
///
/// The new head and the arc type of each bunsetsu.
fn restore_coordination(
    heads: &[Option<usize>],
    conjunct: &[bool],
) -> (Vec<Option<usize>>, Vec<ArcType>) {
    let count = heads.len();
    let mut conjuncts: Vec<Vec<usize>> = vec![Vec::new(); count];
    for index in 0..count {
        if conjunct[index]
            && let Some(head) = heads[index]
        {
            conjuncts[head].push(index);
        }
    }

    let mut new_heads = heads.to_vec();
    let mut arc_types = vec![ArcType::Dependency; count];
    let mut seen = vec![false; count];
    // A stack instead of recursion: (bunsetsu, the head it gets when it has
    // no conjunct of its own, the type of that arc).
    let mut stack: Vec<(usize, Option<usize>, ArcType)> = (0..count)
        .rev()
        .filter(|&index| !conjunct[index])
        .map(|index| (index, heads[index], ArcType::Dependency))
        .collect();
    while let Some((index, target, arc_type)) = stack.pop() {
        // A conjunct is reached once, from its head. Conjuncts that depend
        // on each other in a cycle are never reached, and keep their heads.
        if seen[index] {
            continue;
        }
        seen[index] = true;
        let chain = &conjuncts[index];
        let Some(&first) = chain.first() else {
            new_heads[index] = target;
            arc_types[index] = arc_type;
            continue;
        };
        new_heads[index] = Some(first);
        arc_types[index] = ArcType::Parallel;
        for (position, &conjunct) in chain.iter().enumerate() {
            match chain.get(position + 1) {
                Some(&next) => stack.push((conjunct, Some(next), ArcType::Parallel)),
                None => stack.push((conjunct, target, arc_type)),
            }
        }
    }
    (new_heads, arc_types)
}

/// The causes for which bunsetsu with these heads are dropped.
///
/// # Arguments
///
/// * `heads` - The head of each bunsetsu, `None` for a root.
///
/// # Returns
///
/// [`DropCause::Root`], [`DropCause::HeadToTheLeft`] and
/// [`DropCause::CrossingArcs`], each at most once and in this order; none
/// for a tree as the parsers assume.
pub(super) fn drop_causes(heads: &[Option<usize>]) -> Vec<DropCause> {
    let mut causes = Vec::new();
    for violation in tree_violations(heads) {
        let cause = match violation {
            Violation::NoRoot | Violation::MultipleRoots { .. } | Violation::RootNotLast { .. } => {
                DropCause::Root
            }
            Violation::HeadNotToTheRight { .. } => DropCause::HeadToTheLeft,
            Violation::CrossingArcs { .. } => DropCause::CrossingArcs,
        };
        if !causes.contains(&cause) {
            causes.push(cause);
        }
    }
    causes.sort();
    causes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::tests::{ud_sentence, with_content_heads};

    /// The heads and the arc types of the bunsetsu of a converted sentence,
    /// as `(head, letter)`.
    fn arcs(tree: &BunsetsuTree) -> Vec<(Option<usize>, char)> {
        tree.bunsetsu
            .iter()
            .map(|bunsetsu| (bunsetsu.head, bunsetsu.arc_type.as_char()))
            .collect()
    }

    #[test]
    fn groups_words_into_bunsetsu_and_takes_the_head_from_the_word_that_leaves() {
        // 彼 は | 本 を | 読ん だ
        let sentence = ud_sentence(&[
            ("彼", 5, "nsubj", true),
            ("は", 1, "case", false),
            ("本", 5, "obj", true),
            ("を", 3, "case", false),
            ("読ん", 0, "root", true),
            ("だ", 5, "aux", false),
        ]);
        let tree = to_bunsetsu(&sentence).unwrap();
        let words: Vec<Range<usize>> = tree
            .bunsetsu
            .iter()
            .map(|bunsetsu| bunsetsu.words.clone())
            .collect();
        assert_eq!(words, [0..2, 2..4, 4..6]);
        assert_eq!(arcs(&tree), [(Some(2), 'D'), (Some(2), 'D'), (None, 'D')]);
        assert_eq!(tree.ambiguous_heads, 0);
    }

    #[test]
    fn the_content_head_is_the_one_word_marked_in_the_bunsetsu() {
        // 彼 は | 本 を | 読ん だ
        let words = [
            ("彼", 5, "nsubj", true),
            ("は", 1, "case", false),
            ("本", 5, "obj", true),
            ("を", 3, "case", false),
            ("読ん", 0, "root", true),
            ("だ", 5, "aux", false),
        ];
        let content_heads = |marked: &[usize]| -> Vec<Option<usize>> {
            let sentence = with_content_heads(ud_sentence(&words), marked);
            to_bunsetsu(&sentence)
                .unwrap()
                .bunsetsu
                .iter()
                .map(|bunsetsu| bunsetsu.content_head)
                .collect()
        };
        // One word each: its index in the sentence, the first of the
        // bunsetsu or not.
        assert_eq!(content_heads(&[0, 3, 4]), [Some(0), Some(3), Some(4)]);
        // None in the second bunsetsu, two in the third.
        assert_eq!(content_heads(&[0, 4, 5]), [Some(0), None, None]);
        assert_eq!(content_heads(&[]), [None, None, None]);
    }

    #[test]
    fn two_conjuncts_become_a_chain() {
        // a と b を 読む: UD has b → a (conj) and a → 読む.
        let sentence = ud_sentence(&[
            ("a", 3, "obj", true),
            ("b", 1, "conj", true),
            ("読む", 0, "root", true),
        ]);
        let tree = to_bunsetsu(&sentence).unwrap();
        assert_eq!(arcs(&tree), [(Some(1), 'P'), (Some(2), 'D'), (None, 'D')]);
    }

    #[test]
    fn three_conjuncts_become_a_chain() {
        let sentence = ud_sentence(&[
            ("a", 4, "obj", true),
            ("b", 1, "conj", true),
            ("c", 1, "conj", true),
            ("読む", 0, "root", true),
        ]);
        let tree = to_bunsetsu(&sentence).unwrap();
        assert_eq!(
            arcs(&tree),
            [(Some(1), 'P'), (Some(2), 'P'), (Some(3), 'D'), (None, 'D')]
        );
    }

    #[test]
    fn nested_conjuncts_are_chained_before_the_next_conjunct() {
        // b and d are conjuncts of a; c is a conjunct of b:
        // a → b, b → c, c → d, d → 読む.
        let sentence = ud_sentence(&[
            ("a", 5, "obj", true),
            ("b", 1, "conj", true),
            ("c", 2, "conj", true),
            ("d", 1, "conj", true),
            ("読む", 0, "root", true),
        ]);
        let tree = to_bunsetsu(&sentence).unwrap();
        assert_eq!(
            arcs(&tree),
            [
                (Some(1), 'P'),
                (Some(2), 'P'),
                (Some(3), 'P'),
                (Some(4), 'D'),
                (None, 'D')
            ]
        );
    }

    #[test]
    fn the_last_conjunct_of_the_root_is_the_root() {
        // 読み、書く: UD has the first verb as the root.
        let sentence = ud_sentence(&[("読み", 0, "root", true), ("書く", 1, "conj", true)]);
        let tree = to_bunsetsu(&sentence).unwrap();
        assert_eq!(arcs(&tree), [(Some(1), 'P'), (None, 'D')]);
    }

    #[test]
    fn the_last_word_that_leaves_the_bunsetsu_gives_the_head() {
        // （ 細かい | 字 | 読む: both words of the first bunsetsu leave it.
        let sentence = ud_sentence(&[
            ("（", 4, "punct", true),
            ("細かい", 3, "acl", false),
            ("字", 4, "obj", true),
            ("読む", 0, "root", true),
        ]);
        let tree = to_bunsetsu(&sentence).unwrap();
        assert_eq!(arcs(&tree), [(Some(1), 'D'), (Some(2), 'D'), (None, 'D')]);
        assert_eq!(tree.ambiguous_heads, 1);
    }

    #[test]
    fn drops_a_sentence_whose_first_word_starts_no_bunsetsu() {
        let sentence = ud_sentence(&[("彼", 2, "nsubj", false), ("読む", 0, "root", true)]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::Bunsetsu]));
        assert_eq!(
            to_bunsetsu(&ud_sentence(&[])),
            Err(vec![DropCause::Bunsetsu])
        );
    }

    #[test]
    fn drops_a_sentence_with_a_bunsetsu_no_word_leaves() {
        // The two words of the first bunsetsu depend on each other.
        let sentence = ud_sentence(&[
            ("彼", 2, "dep", true),
            ("は", 1, "dep", false),
            ("読む", 0, "root", true),
        ]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::Bunsetsu]));
    }

    #[test]
    fn drops_a_sentence_whose_root_is_not_the_last_bunsetsu() {
        let sentence = ud_sentence(&[("読む", 0, "root", true), ("彼", 1, "nsubj", true)]);
        assert_eq!(
            to_bunsetsu(&sentence),
            Err(vec![DropCause::Root, DropCause::HeadToTheLeft])
        );
        // Two roots, the last bunsetsu among them.
        let sentence = ud_sentence(&[("はい", 0, "root", true), ("読む", 0, "root", true)]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::Root]));
    }

    #[test]
    fn drops_a_sentence_with_a_head_to_the_left() {
        // An arc to the left that is not a conjunct stays one.
        let sentence = ud_sentence(&[
            ("彼", 3, "nsubj", true),
            ("本", 1, "dislocated", true),
            ("読む", 0, "root", true),
        ]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::HeadToTheLeft]));
    }

    #[test]
    fn drops_a_sentence_with_crossing_arcs() {
        // a → c and b → d cross.
        let sentence = ud_sentence(&[
            ("a", 3, "dep", true),
            ("b", 4, "dep", true),
            ("c", 4, "dep", true),
            ("d", 0, "root", true),
        ]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::CrossingArcs]));
    }

    #[test]
    fn conjuncts_in_a_cycle_keep_their_heads_and_drop_the_sentence() {
        // a and b are conjuncts of each other: neither is reached.
        let sentence = ud_sentence(&[
            ("a", 2, "conj", true),
            ("b", 1, "conj", true),
            ("読む", 0, "root", true),
        ]);
        assert_eq!(to_bunsetsu(&sentence), Err(vec![DropCause::HeadToTheLeft]));
    }
}

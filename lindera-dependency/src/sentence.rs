//! A sentence: morphemes, grouped into bunsetsu with a head each.

use std::fmt;
use std::ops::Range;

use crate::error::{Error, Result};

/// The type of the arc from a bunsetsu to its head: the letter that follows
/// the head in the lattice format, as in the Kyoto University Text Corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ArcType {
    /// A plain dependency (`D`).
    #[default]
    Dependency,
    /// Coordination (`P`): the bunsetsu and its head are conjuncts.
    Parallel,
    /// Apposition (`A`).
    Apposition,
    /// Incomplete coordination (`I`).
    Incomplete,
}

impl ArcType {
    /// The letter of the arc type in the lattice format.
    ///
    /// # Returns
    ///
    /// `D`, `P`, `A` or `I`.
    pub fn as_char(self) -> char {
        match self {
            ArcType::Dependency => 'D',
            ArcType::Parallel => 'P',
            ArcType::Apposition => 'A',
            ArcType::Incomplete => 'I',
        }
    }

    /// The arc type of a letter of the lattice format.
    ///
    /// # Arguments
    ///
    /// * `letter` - The letter that follows a head.
    ///
    /// # Returns
    ///
    /// The arc type, or `None` for a letter that is not one.
    pub fn from_char(letter: char) -> Option<Self> {
        match letter {
            'D' => Some(ArcType::Dependency),
            'P' => Some(ArcType::Parallel),
            'A' => Some(ArcType::Apposition),
            'I' => Some(ArcType::Incomplete),
            _ => None,
        }
    }
}

impl fmt::Display for ArcType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

/// A bunsetsu (文節): a run of morphemes of a sentence, and the bunsetsu it
/// depends on.
#[derive(Debug, Clone, PartialEq)]
pub struct Bunsetsu {
    /// The morphemes of the bunsetsu, as a range of indices into
    /// [`Sentence::morphemes`].
    pub morphemes: Range<usize>,
    /// The index of the bunsetsu this one depends on, or `None` for the root
    /// (`-1` in the lattice format).
    pub head: Option<usize>,
    /// The type of the arc to the head.
    pub arc_type: ArcType,
    /// The positions of the head word and of the function word, counted from
    /// the first morpheme of the bunsetsu; `None` when they are not known
    /// (a bunsetsu line with the head only).
    pub positions: Option<(usize, usize)>,
    /// The score of the arc, or `None` when there is none.
    pub score: Option<f64>,
}

impl Bunsetsu {
    /// Makes a bunsetsu with a plain dependency, without positions or score.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the bunsetsu, as a range of indices.
    /// * `head` - The index of the head bunsetsu, or `None` for the root.
    ///
    /// # Returns
    ///
    /// The bunsetsu.
    pub fn new(morphemes: Range<usize>, head: Option<usize>) -> Self {
        Self {
            morphemes,
            head,
            arc_type: ArcType::Dependency,
            positions: None,
            score: None,
        }
    }
}

/// A way a sentence breaks the tree the parsers assume: one root, which is
/// the last bunsetsu, every head to the right of its bunsetsu, and no
/// crossing arcs. See [`Sentence::violations`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Violation {
    /// No bunsetsu is a root.
    NoRoot,
    /// More than one bunsetsu is a root.
    MultipleRoots {
        /// The indices of the roots.
        roots: Vec<usize>,
    },
    /// The last bunsetsu is not a root, though another one is.
    RootNotLast {
        /// The index of the first root.
        root: usize,
    },
    /// The head of a bunsetsu is the bunsetsu itself or one to its left.
    HeadNotToTheRight {
        /// The index of the bunsetsu.
        bunsetsu: usize,
        /// The index of its head.
        head: usize,
    },
    /// Two arcs cross: the second bunsetsu lies between the first and its
    /// head, and its own head lies beyond that head.
    CrossingArcs {
        /// The index of the bunsetsu to the left.
        first: usize,
        /// The index of the bunsetsu to the right.
        second: usize,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::NoRoot => write!(f, "no bunsetsu is a root"),
            Violation::MultipleRoots { roots } => {
                write!(f, "{} bunsetsu are roots: {roots:?}", roots.len())
            }
            Violation::RootNotLast { root } => {
                write!(f, "the root is bunsetsu {root}, not the last one")
            }
            Violation::HeadNotToTheRight { bunsetsu, head } => {
                write!(
                    f,
                    "the head of bunsetsu {bunsetsu} is {head}, not to its right"
                )
            }
            Violation::CrossingArcs { first, second } => {
                write!(f, "the arcs of bunsetsu {first} and {second} cross")
            }
        }
    }
}

/// A sentence: its morphemes, and the bunsetsu that group them.
///
/// A sentence without bunsetsu holds morphemes only, as the lattice format
/// does without `*` lines. The fields are public; [`Sentence::new`] and
/// [`Sentence::check`] tell whether they fit together.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentence<M> {
    /// The comment lines before the sentence in the lattice format, each
    /// with its leading `#`.
    pub comments: Vec<String>,
    /// The morphemes of the sentence.
    pub morphemes: Vec<M>,
    /// The bunsetsu of the sentence, from left to right; empty for a
    /// sentence of morphemes only.
    pub bunsetsu: Vec<Bunsetsu>,
}

impl<M> Sentence<M> {
    /// Makes a sentence without comments, after checking its structure.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the sentence.
    /// * `bunsetsu` - Its bunsetsu, from left to right, or none.
    ///
    /// # Returns
    ///
    /// The sentence, or [`Error::Structure`] when [`Sentence::check`] fails.
    pub fn new(morphemes: Vec<M>, bunsetsu: Vec<Bunsetsu>) -> Result<Self> {
        let sentence = Self {
            comments: Vec::new(),
            morphemes,
            bunsetsu,
        };
        sentence.check()?;
        Ok(sentence)
    }

    /// Checks that the sentence is one the lattice format can hold.
    ///
    /// The bunsetsu, when there are any, cover the morphemes from the first
    /// to the last without a gap or an overlap, and none is empty; a head is
    /// the index of a bunsetsu; the positions of the head word and of the
    /// function word are inside the bunsetsu; a score has positions with it,
    /// and is finite; a comment starts with `#` and holds no tab or line
    /// break.
    ///
    /// The tree is not checked here: see [`Sentence::violations`].
    ///
    /// # Returns
    ///
    /// `Ok(())`, or [`Error::Structure`] for the first thing that is wrong.
    pub fn check(&self) -> Result<()> {
        check_comments(&self.comments)?;
        check_bunsetsu(self.morphemes.len(), &self.bunsetsu)
    }

    /// The ways the sentence breaks the tree the parsers assume: one root,
    /// the root is the last bunsetsu, every head is to the right of its
    /// bunsetsu, and no two arcs cross.
    ///
    /// The corpus conversion and the evaluation share this check, to count
    /// and to leave out the sentences that break it.
    ///
    /// # Returns
    ///
    /// Every violation, in the order of the list above and then of the
    /// bunsetsu; none for a valid tree, and none for a sentence without
    /// bunsetsu. A head that is not the index of a bunsetsu is not reported
    /// here, but by [`Sentence::check`].
    pub fn violations(&self) -> Vec<Violation> {
        let mut violations = Vec::new();
        let count = self.bunsetsu.len();
        if count == 0 {
            return violations;
        }

        let roots: Vec<usize> = (0..count)
            .filter(|&index| self.bunsetsu[index].head.is_none())
            .collect();
        match roots.first() {
            None => violations.push(Violation::NoRoot),
            Some(&root) => {
                if roots.len() > 1 {
                    violations.push(Violation::MultipleRoots {
                        roots: roots.clone(),
                    });
                }
                if self.bunsetsu[count - 1].head.is_some() {
                    violations.push(Violation::RootNotLast { root });
                }
            }
        }

        for (index, bunsetsu) in self.bunsetsu.iter().enumerate() {
            if let Some(head) = bunsetsu.head
                && head <= index
            {
                violations.push(Violation::HeadNotToTheRight {
                    bunsetsu: index,
                    head,
                });
            }
        }

        // Among the arcs that go to the right, two cross when one starts
        // inside the other and ends outside it.
        for (first, bunsetsu) in self.bunsetsu.iter().enumerate() {
            let Some(first_head) = bunsetsu.head.filter(|&head| head > first) else {
                continue;
            };
            let end = first_head.min(count);
            for second in first + 1..end {
                if let Some(second_head) = self.bunsetsu[second].head
                    && second_head > first_head
                {
                    violations.push(Violation::CrossingArcs { first, second });
                }
            }
        }

        violations
    }
}

/// Checks the comment lines of a sentence.
///
/// # Arguments
///
/// * `comments` - The comment lines.
///
/// # Returns
///
/// `Ok(())`, or [`Error::Structure`] for a comment that does not start with
/// `#`, or that holds a tab or a line break.
fn check_comments(comments: &[String]) -> Result<()> {
    for comment in comments {
        if !comment.starts_with('#') {
            return Err(Error::Structure(format!(
                "the comment {comment:?} does not start with `#`"
            )));
        }
        if comment.contains(['\t', '\n']) {
            return Err(Error::Structure(format!(
                "the comment {comment:?} holds a tab or a line break"
            )));
        }
    }
    Ok(())
}

/// Checks that bunsetsu fit a number of morphemes; see [`Sentence::check`].
///
/// # Arguments
///
/// * `morphemes` - The number of morphemes of the sentence.
/// * `bunsetsu` - The bunsetsu of the sentence.
///
/// # Returns
///
/// `Ok(())`, or [`Error::Structure`] for the first thing that is wrong.
fn check_bunsetsu(morphemes: usize, bunsetsu: &[Bunsetsu]) -> Result<()> {
    if bunsetsu.is_empty() {
        return Ok(());
    }
    let mut next = 0;
    for (index, one) in bunsetsu.iter().enumerate() {
        let Range { start, end } = one.morphemes;
        if start != next {
            return Err(Error::Structure(format!(
                "bunsetsu {index} starts at morpheme {start}, not at {next}"
            )));
        }
        if end <= start {
            return Err(Error::Structure(format!(
                "bunsetsu {index} has no morpheme"
            )));
        }
        next = end;

        if let Some(head) = one.head
            && head >= bunsetsu.len()
        {
            return Err(Error::Structure(format!(
                "the head of bunsetsu {index} is {head}, but there are {} bunsetsu",
                bunsetsu.len()
            )));
        }
        let size = end - start;
        if let Some((head_word, function_word)) = one.positions
            && (head_word >= size || function_word >= size)
        {
            return Err(Error::Structure(format!(
                "the positions {head_word}/{function_word} of bunsetsu {index} are outside its {size} morpheme(s)"
            )));
        }
        if let Some(score) = one.score {
            if one.positions.is_none() {
                return Err(Error::Structure(format!(
                    "bunsetsu {index} has a score without positions"
                )));
            }
            if !score.is_finite() {
                return Err(Error::Structure(format!(
                    "the score of bunsetsu {index} is not finite"
                )));
            }
        }
    }
    if next != morphemes {
        return Err(Error::Structure(format!(
            "the bunsetsu end at morpheme {next}, but there are {morphemes} morpheme(s)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sentence of one-morpheme bunsetsu with the given heads.
    fn tree(heads: &[Option<usize>]) -> Sentence<()> {
        Sentence {
            comments: Vec::new(),
            morphemes: vec![(); heads.len()],
            bunsetsu: heads
                .iter()
                .enumerate()
                .map(|(index, &head)| Bunsetsu::new(index..index + 1, head))
                .collect(),
        }
    }

    fn structure_error(sentence: &Sentence<()>) -> String {
        match sentence.check() {
            Err(Error::Structure(message)) => message,
            other => panic!("expected a structure error, got {other:?}"),
        }
    }

    #[test]
    fn arc_type_letters() {
        for arc_type in [
            ArcType::Dependency,
            ArcType::Parallel,
            ArcType::Apposition,
            ArcType::Incomplete,
        ] {
            assert_eq!(ArcType::from_char(arc_type.as_char()), Some(arc_type));
        }
        assert_eq!(ArcType::Parallel.to_string(), "P");
        assert_eq!(ArcType::from_char('X'), None);
        assert_eq!(ArcType::from_char('d'), None);
        assert_eq!(ArcType::default(), ArcType::Dependency);
    }

    #[test]
    fn a_valid_tree_has_no_violation() {
        assert!(tree(&[Some(2), Some(2), None]).violations().is_empty());
        assert!(
            tree(&[Some(1), Some(2), Some(3), None])
                .violations()
                .is_empty()
        );
        assert!(
            tree(&[Some(3), Some(2), Some(3), None])
                .violations()
                .is_empty()
        );
        assert!(tree(&[None]).violations().is_empty());
        assert!(tree(&[]).violations().is_empty());
    }

    #[test]
    fn reports_no_root() {
        assert_eq!(
            tree(&[Some(1), Some(2), Some(0)]).violations(),
            [
                Violation::NoRoot,
                Violation::HeadNotToTheRight {
                    bunsetsu: 2,
                    head: 0
                },
            ]
        );
    }

    #[test]
    fn reports_multiple_roots() {
        assert_eq!(
            tree(&[None, Some(2), None]).violations(),
            [Violation::MultipleRoots { roots: vec![0, 2] }]
        );
    }

    #[test]
    fn reports_a_root_that_is_not_last() {
        assert_eq!(
            tree(&[Some(1), None, Some(1)]).violations(),
            [
                Violation::RootNotLast { root: 1 },
                Violation::HeadNotToTheRight {
                    bunsetsu: 2,
                    head: 1
                },
            ]
        );
    }

    #[test]
    fn reports_a_head_that_is_not_to_the_right() {
        assert_eq!(
            tree(&[Some(2), Some(1), None]).violations(),
            [Violation::HeadNotToTheRight {
                bunsetsu: 1,
                head: 1
            }]
        );
        assert_eq!(
            tree(&[Some(2), Some(0), None]).violations(),
            [Violation::HeadNotToTheRight {
                bunsetsu: 1,
                head: 0
            }]
        );
    }

    #[test]
    fn reports_crossing_arcs() {
        // 0 -> 2 and 1 -> 3 cross.
        assert_eq!(
            tree(&[Some(2), Some(3), Some(3), None]).violations(),
            [Violation::CrossingArcs {
                first: 0,
                second: 1
            }]
        );
        // 1 -> 3 and 2 -> 4 cross; 0 -> 4 encloses both.
        assert_eq!(
            tree(&[Some(4), Some(3), Some(4), Some(4), None]).violations(),
            [Violation::CrossingArcs {
                first: 1,
                second: 2
            }]
        );
    }

    #[test]
    fn nested_arcs_and_arcs_that_share_a_head_do_not_cross() {
        assert!(
            tree(&[Some(3), Some(2), Some(3), None])
                .violations()
                .is_empty()
        );
        assert!(
            tree(&[Some(2), Some(2), Some(3), None])
                .violations()
                .is_empty()
        );
    }

    #[test]
    fn violations_do_not_panic_on_a_head_out_of_range() {
        let sentence = tree(&[Some(7), Some(2), None]);
        assert!(sentence.violations().is_empty());
        assert!(structure_error(&sentence).contains("the head of bunsetsu 0 is 7"));
    }

    #[test]
    fn new_checks_the_structure() {
        let sentence = Sentence::new(
            vec![(); 3],
            vec![Bunsetsu::new(0..2, Some(1)), Bunsetsu::new(2..3, None)],
        );
        assert!(sentence.is_ok());
        let sentence = Sentence::new(vec![(); 3], vec![Bunsetsu::new(0..2, None)]);
        assert!(matches!(sentence, Err(Error::Structure(_))));
        let sentence = Sentence::new(vec![(); 3], Vec::new());
        assert!(sentence.is_ok());
    }

    #[test]
    fn check_refuses_bunsetsu_that_do_not_cover_the_morphemes() {
        let mut sentence = tree(&[Some(1), None]);
        sentence.bunsetsu[1].morphemes = 2..3;
        assert_eq!(
            structure_error(&sentence),
            "bunsetsu 1 starts at morpheme 2, not at 1"
        );

        let mut sentence = tree(&[Some(1), None]);
        sentence.bunsetsu[0].morphemes = 0..0;
        assert_eq!(structure_error(&sentence), "bunsetsu 0 has no morpheme");

        let mut sentence = tree(&[Some(1), None]);
        sentence.morphemes.push(());
        assert_eq!(
            structure_error(&sentence),
            "the bunsetsu end at morpheme 2, but there are 3 morpheme(s)"
        );
    }

    #[test]
    fn check_refuses_positions_outside_the_bunsetsu() {
        let mut sentence = tree(&[Some(1), None]);
        sentence.bunsetsu[0].positions = Some((0, 0));
        assert!(sentence.check().is_ok());
        sentence.bunsetsu[0].positions = Some((1, 0));
        assert!(structure_error(&sentence).contains("positions 1/0 of bunsetsu 0"));
        sentence.bunsetsu[0].positions = Some((0, 1));
        assert!(structure_error(&sentence).contains("positions 0/1 of bunsetsu 0"));
    }

    #[test]
    fn check_refuses_a_score_without_positions_or_not_finite() {
        let mut sentence = tree(&[Some(1), None]);
        sentence.bunsetsu[0].score = Some(1.0);
        assert_eq!(
            structure_error(&sentence),
            "bunsetsu 0 has a score without positions"
        );
        sentence.bunsetsu[0].positions = Some((0, 0));
        assert!(sentence.check().is_ok());
        sentence.bunsetsu[0].score = Some(f64::NAN);
        assert_eq!(
            structure_error(&sentence),
            "the score of bunsetsu 0 is not finite"
        );
    }

    #[test]
    fn check_refuses_a_bad_comment() {
        let mut sentence = tree(&[None]);
        sentence.comments = vec!["# S-ID: 1".to_string()];
        assert!(sentence.check().is_ok());
        sentence.comments = vec!["S-ID: 1".to_string()];
        assert!(structure_error(&sentence).contains("does not start with `#`"));
        sentence.comments = vec!["# a\tb".to_string()];
        assert!(structure_error(&sentence).contains("holds a tab or a line break"));
    }

    #[test]
    fn violations_display() {
        assert_eq!(
            Violation::CrossingArcs {
                first: 0,
                second: 1
            }
            .to_string(),
            "the arcs of bunsetsu 0 and 1 cross"
        );
    }
}

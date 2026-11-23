//! Evaluating chunking and dependency accuracy, as CaboCha and J.DepP do.
//!
//! [`evaluate`] compares the output of a chunker or a parser with the
//! answer, sentence by sentence, and counts three groups of numbers. Every
//! number is kept as a numerator and a denominator ([`Count`]).
//!
//! - **Chunking**: precision, recall and F over bunsetsu. A bunsetsu of the
//!   output is right when a bunsetsu of the answer starts at the same
//!   morpheme and has the same length. This is `eval_chunk` of CaboCha.
//! - **Dependency with the same segmentation**: on the sentences whose
//!   segmentation is the one of the answer, the bunsetsu whose head is
//!   right. Level 0 counts every bunsetsu, level 1 every one but the last of
//!   its sentence, level 2 every one but the last two. This is `eval_dep` of
//!   CaboCha; level 1 is the number J.DepP reports.
//! - **End to end**: over the bunsetsu of the answer but the last of each
//!   sentence, the ones for which the output has a bunsetsu with the same
//!   span, whose head has the span of the head in the answer.
//!
//! The arc type is not compared. A sentence whose morphemes differ between
//! the two files is left out of every count and reported.
//!
//! [`compare`] evaluates two outputs for the same answer and pairs them arc
//! by arc, for McNemar's test ([`mcnemar_exact`]). [`fold_of`] splits a file
//! into the folds of a cross-validation.
//!
//! # Examples
//!
//! ```
//! use lindera_dependency::{evaluation, lattice};
//!
//! let gold = lattice::parse("\
//! * 0 2D \n彼\t*\nは\t*\n\
//! * 1 2D \n本\t*\nを\t*\n\
//! * 2 -1D \n読む\t*\n\
//! EOS\n")?;
//! // The output has the same bunsetsu, and a wrong head for the first one.
//! let system = lattice::parse("\
//! * 0 1D \n彼\t*\nは\t*\n\
//! * 1 2D \n本\t*\nを\t*\n\
//! * 2 -1D \n読む\t*\n\
//! EOS\n")?;
//!
//! let evaluation = evaluation::evaluate(&gold, &system)?;
//! assert_eq!(evaluation.chunking.f_score(), Some(1.0));
//! assert_eq!(evaluation.dependency.level1.correct, 1);
//! assert_eq!(evaluation.dependency.level1.total, 2);
//! assert_eq!(evaluation.end_to_end.arcs.ratio(), Some(0.5));
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

use std::fmt;
use std::num::NonZeroUsize;
use std::ops::Range;

use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::sentence::Sentence;

/// A count of what is right among what is counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    /// The number that are right.
    pub correct: usize,
    /// The number counted.
    pub total: usize,
}

impl Count {
    /// Counts one more.
    ///
    /// # Arguments
    ///
    /// * `correct` - Whether it is right.
    fn add(&mut self, correct: bool) {
        self.total += 1;
        if correct {
            self.correct += 1;
        }
    }

    /// The share that is right.
    ///
    /// # Returns
    ///
    /// `correct / total`, or `None` when nothing was counted.
    pub fn ratio(&self) -> Option<f64> {
        ratio(self.correct, self.total)
    }
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}/{})",
            Percent(self.ratio()),
            self.correct,
            self.total
        )
    }
}

/// A share of a whole.
///
/// # Arguments
///
/// * `part` - The part.
/// * `whole` - The whole.
///
/// # Returns
///
/// `part / whole`, or `None` when the whole is 0.
fn ratio(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| part as f64 / whole as f64)
}

/// A ratio written as a percentage with two decimals, or `n/a`.
struct Percent(Option<f64>);

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(ratio) => write!(f, "{:.2}%", 100.0 * ratio),
            None => f.write_str("n/a"),
        }
    }
}

/// The counts of chunking.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Chunking {
    /// The bunsetsu of the output that are bunsetsu of the answer: the same
    /// first morpheme and the same length.
    pub correct: usize,
    /// The bunsetsu of the output.
    pub system: usize,
    /// The bunsetsu of the answer.
    pub gold: usize,
    /// The sentences with every bunsetsu right.
    pub sentences: Count,
}

impl Chunking {
    /// The precision.
    ///
    /// # Returns
    ///
    /// `correct / system`, or `None` when the output has no bunsetsu.
    pub fn precision(&self) -> Option<f64> {
        ratio(self.correct, self.system)
    }

    /// The recall.
    ///
    /// # Returns
    ///
    /// `correct / gold`, or `None` when the answer has no bunsetsu.
    pub fn recall(&self) -> Option<f64> {
        ratio(self.correct, self.gold)
    }

    /// The F score: the harmonic mean of the precision and the recall.
    ///
    /// # Returns
    ///
    /// The score, 0 when no bunsetsu is right, or `None` when the precision
    /// or the recall is not defined.
    pub fn f_score(&self) -> Option<f64> {
        let precision = self.precision()?;
        let recall = self.recall()?;
        if precision + recall == 0.0 {
            Some(0.0)
        } else {
            Some(2.0 * precision * recall / (precision + recall))
        }
    }
}

/// The counts of dependency, on the sentences whose segmentation is the one
/// of the answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dependency {
    /// Every bunsetsu.
    pub level0: Count,
    /// Every bunsetsu but the last of its sentence.
    pub level1: Count,
    /// Every bunsetsu but the last two of its sentence.
    pub level2: Count,
    /// The sentences with every head right.
    pub sentences: Count,
}

/// The counts from end to end: segmentation and heads together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EndToEnd {
    /// The bunsetsu of the answer but the last of each sentence. One is
    /// right when the output has a bunsetsu with the same span, whose head
    /// has the span of the head in the answer.
    pub arcs: Count,
    /// The sentences with the segmentation and every head right.
    pub sentences: Count,
}

/// The evaluation of an output against an answer.
///
/// [`fmt::Display`] writes it as lines, each ratio with its numerator and
/// its denominator.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Evaluation {
    /// The sentences of the answer.
    pub sentences: usize,
    /// The sentences left out because their morphemes differ between the
    /// answer and the output, by their number in the file, from 1.
    pub different: Vec<usize>,
    /// The sentences not counted because the answer has no bunsetsu for
    /// them.
    pub without_bunsetsu: usize,
    /// Chunking.
    pub chunking: Chunking,
    /// Dependency with the same segmentation.
    pub dependency: Dependency,
    /// End to end.
    pub end_to_end: EndToEnd,
}

impl fmt::Display for Evaluation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "sentences: {}", self.sentences)?;
        writeln!(
            f,
            "left out, different morphemes: {}{}",
            self.different.len(),
            Numbers(&self.different)
        )?;
        writeln!(f, "not counted, no bunsetsu: {}", self.without_bunsetsu)?;
        let chunking = &self.chunking;
        writeln!(
            f,
            "chunking precision: {} ({}/{})",
            Percent(chunking.precision()),
            chunking.correct,
            chunking.system
        )?;
        writeln!(
            f,
            "chunking recall: {} ({}/{})",
            Percent(chunking.recall()),
            chunking.correct,
            chunking.gold
        )?;
        writeln!(f, "chunking F: {}", Percent(chunking.f_score()))?;
        writeln!(f, "chunking sentences: {}", chunking.sentences)?;
        writeln!(f, "dependency level 0: {}", self.dependency.level0)?;
        writeln!(f, "dependency level 1: {}", self.dependency.level1)?;
        writeln!(f, "dependency level 2: {}", self.dependency.level2)?;
        writeln!(f, "dependency sentences: {}", self.dependency.sentences)?;
        writeln!(f, "end to end: {}", self.end_to_end.arcs)?;
        write!(f, "end to end sentences: {}", self.end_to_end.sentences)
    }
}

/// The numbers of sentences, as ` (1, 2, 3)`: the first ten, and how many
/// more; nothing for none.
struct Numbers<'a>(&'a [usize]);

impl fmt::Display for Numbers<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SHOWN: usize = 10;
        if self.0.is_empty() {
            return Ok(());
        }
        f.write_str(" (sentences ")?;
        for (index, number) in self.0.iter().take(SHOWN).enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{number}")?;
        }
        if self.0.len() > SHOWN {
            write!(f, " and {} more", self.0.len() - SHOWN)?;
        }
        f.write_str(")")
    }
}

/// What one sentence adds to an evaluation.
struct Outcome {
    /// Whether each bunsetsu of the answer but the last is right from end to
    /// end.
    arcs: Vec<bool>,
    /// Whether the segmentation and every head are right.
    sentence: bool,
}

/// Evaluates an output against an answer.
///
/// The sentences correspond by their order. Both have to be sentences the
/// lattice format can hold ([`Sentence::check`]); neither has to be a tree.
///
/// # Arguments
///
/// * `gold` - The sentences of the answer.
/// * `system` - The sentences of the output, as many.
///
/// # Returns
///
/// The evaluation, or an error: [`Error::Evaluation`] when the numbers of
/// sentences differ, [`Error::Structure`] for a sentence that
/// [`Sentence::check`] refuses.
pub fn evaluate<G: Morpheme, S: Morpheme>(
    gold: &[Sentence<G>],
    system: &[Sentence<S>],
) -> Result<Evaluation> {
    let mut evaluation = Evaluation::default();
    evaluate_into(&mut evaluation, gold, system)?;
    Ok(evaluation)
}

/// Evaluates an output against an answer, and keeps what each sentence adds.
///
/// # Arguments
///
/// * `evaluation` - The evaluation to fill.
/// * `gold` - The sentences of the answer.
/// * `system` - The sentences of the output.
///
/// # Returns
///
/// For each sentence, its outcome, or `None` when it was not counted; or
/// the error of [`evaluate`].
fn evaluate_into<G: Morpheme, S: Morpheme>(
    evaluation: &mut Evaluation,
    gold: &[Sentence<G>],
    system: &[Sentence<S>],
) -> Result<Vec<Option<Outcome>>> {
    if gold.len() != system.len() {
        return Err(Error::Evaluation(format!(
            "the answer has {} sentences and the output {}",
            gold.len(),
            system.len()
        )));
    }
    let mut outcomes = Vec::with_capacity(gold.len());
    for (index, (gold, system)) in gold.iter().zip(system).enumerate() {
        gold.check()?;
        system.check()?;
        evaluation.sentences += 1;
        if !same_morphemes(gold, system) {
            evaluation.different.push(index + 1);
            outcomes.push(None);
            continue;
        }
        if gold.bunsetsu.is_empty() {
            evaluation.without_bunsetsu += 1;
            outcomes.push(None);
            continue;
        }
        outcomes.push(Some(evaluate_sentence(evaluation, gold, system)));
    }
    Ok(outcomes)
}

/// Whether two sentences have the same morphemes: as many, with the same
/// surfaces.
///
/// # Arguments
///
/// * `gold` - The sentence of the answer.
/// * `system` - The sentence of the output.
///
/// # Returns
///
/// `true` when the surfaces are the same, one by one.
fn same_morphemes<G: Morpheme, S: Morpheme>(gold: &Sentence<G>, system: &Sentence<S>) -> bool {
    gold.morphemes.len() == system.morphemes.len()
        && gold
            .morphemes
            .iter()
            .zip(&system.morphemes)
            .all(|(gold, system)| gold.surface() == system.surface())
}

/// Counts one sentence with the same morphemes in the answer and the
/// output, and bunsetsu in the answer.
///
/// # Arguments
///
/// * `evaluation` - The evaluation to add to.
/// * `gold` - The sentence of the answer.
/// * `system` - The sentence of the output.
///
/// # Returns
///
/// What the sentence adds from end to end.
fn evaluate_sentence<G, S>(
    evaluation: &mut Evaluation,
    gold: &Sentence<G>,
    system: &Sentence<S>,
) -> Outcome {
    let gold = &gold.bunsetsu;
    let system = &system.bunsetsu;

    // Chunking, as `eval_chunk` of CaboCha: the bunsetsu of the two are
    // walked together by their first morpheme.
    let chunking = &mut evaluation.chunking;
    chunking.system += system.len();
    chunking.gold += gold.len();
    let (mut system_index, mut gold_index) = (0, 0);
    let mut correct = 0;
    while system_index < system.len() && gold_index < gold.len() {
        let system_span = &system[system_index].morphemes;
        let gold_span = &gold[gold_index].morphemes;
        if system_span.start == gold_span.start {
            if system_span.end == gold_span.end {
                correct += 1;
            }
            system_index += 1;
            gold_index += 1;
        } else if system_span.start < gold_span.start {
            system_index += 1;
        } else {
            gold_index += 1;
        }
    }
    chunking.correct += correct;
    // Every bunsetsu of the answer is one of the output: as both cover the
    // same morphemes, the output has no other bunsetsu.
    let same_segmentation = correct == gold.len();
    chunking.sentences.add(same_segmentation);

    // Dependency, as `eval_dep` of CaboCha, on a sentence with the
    // segmentation of the answer: heads are compared by index.
    let mut every_head = same_segmentation;
    if same_segmentation {
        let dependency = &mut evaluation.dependency;
        let count = gold.len();
        for (index, (gold, system)) in gold.iter().zip(system).enumerate() {
            let right = gold.head == system.head;
            every_head &= right;
            dependency.level0.add(right);
            if index + 1 < count {
                dependency.level1.add(right);
            }
            if index + 2 < count {
                dependency.level2.add(right);
            }
        }
        dependency.sentences.add(every_head);
    }

    // End to end: for each bunsetsu of the answer but the last, the output
    // has a bunsetsu with the same span, and the spans of the two heads are
    // the same.
    let span_of_head = |bunsetsu: &[crate::Bunsetsu], index: usize| -> Option<Range<usize>> {
        bunsetsu[index]
            .head
            .map(|head| bunsetsu[head].morphemes.clone())
    };
    let mut arcs = Vec::with_capacity(gold.len().saturating_sub(1));
    for (index, bunsetsu) in gold.iter().enumerate().take(gold.len() - 1) {
        let right = system
            .binary_search_by_key(&bunsetsu.morphemes.start, |other| other.morphemes.start)
            .is_ok_and(|found| {
                system[found].morphemes == bunsetsu.morphemes
                    && span_of_head(system, found) == span_of_head(gold, index)
            });
        evaluation.end_to_end.arcs.add(right);
        arcs.push(right);
    }
    evaluation.end_to_end.sentences.add(every_head);

    Outcome {
        arcs,
        sentence: every_head,
    }
}

/// The paired outcomes of two outputs: how many are right in both, in one
/// only, or in neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pairs {
    /// Right in both outputs.
    pub both: usize,
    /// Right in the first output only.
    pub first_only: usize,
    /// Right in the second output only.
    pub second_only: usize,
    /// Right in neither output.
    pub neither: usize,
}

impl Pairs {
    /// Counts one more pair.
    ///
    /// # Arguments
    ///
    /// * `first` - Whether the first output is right.
    /// * `second` - Whether the second output is right.
    fn add(&mut self, first: bool, second: bool) {
        match (first, second) {
            (true, true) => self.both += 1,
            (true, false) => self.first_only += 1,
            (false, true) => self.second_only += 1,
            (false, false) => self.neither += 1,
        }
    }

    /// The p-value of McNemar's exact test on the pairs.
    ///
    /// # Returns
    ///
    /// [`mcnemar_exact`] of the two discordant counts.
    pub fn p_value(&self) -> f64 {
        mcnemar_exact(self.first_only, self.second_only)
    }
}

impl fmt::Display for Pairs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "both {}, first only {}, second only {}, neither {}, McNemar p = {:.4}",
            self.both,
            self.first_only,
            self.second_only,
            self.neither,
            self.p_value()
        )
    }
}

/// Two outputs for the same answer, each evaluated, and paired.
///
/// The pairs are over the sentences counted in both evaluations: a sentence
/// whose morphemes differ from the answer in either output is in neither
/// kind of pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comparison {
    /// The evaluation of the first output.
    pub first: Evaluation,
    /// The evaluation of the second output.
    pub second: Evaluation,
    /// The pairs of end-to-end arcs: the bunsetsu of the answer but the last
    /// of each sentence. With the segmentation of the answer in both
    /// outputs, these are the arcs of level 1.
    pub arcs: Pairs,
    /// The pairs of sentences, right when the segmentation and every head
    /// are.
    pub sentences: Pairs,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "first output\n{}\n", self.first)?;
        writeln!(f, "second output\n{}\n", self.second)?;
        writeln!(f, "paired arcs: {}", self.arcs)?;
        write!(f, "paired sentences: {}", self.sentences)
    }
}

/// Evaluates two outputs against the same answer, and pairs them.
///
/// # Arguments
///
/// * `gold` - The sentences of the answer.
/// * `first` - The sentences of the first output, as many.
/// * `second` - The sentences of the second output, as many.
///
/// # Returns
///
/// The comparison, or the error of [`evaluate`] for either output.
pub fn compare<G: Morpheme, A: Morpheme, B: Morpheme>(
    gold: &[Sentence<G>],
    first: &[Sentence<A>],
    second: &[Sentence<B>],
) -> Result<Comparison> {
    let mut comparison = Comparison::default();
    let first = evaluate_into(&mut comparison.first, gold, first)?;
    let second = evaluate_into(&mut comparison.second, gold, second)?;
    for (first, second) in first.iter().zip(&second) {
        let (Some(first), Some(second)) = (first, second) else {
            continue;
        };
        for (&first, &second) in first.arcs.iter().zip(&second.arcs) {
            comparison.arcs.add(first, second);
        }
        comparison.sentences.add(first.sentence, second.sentence);
    }
    Ok(comparison)
}

/// The p-value of McNemar's exact test.
///
/// Under the hypothesis that the two outputs are right equally often, each
/// discordant pair is right in the first only or in the second only with the
/// probability 1/2. The p-value is the probability of a split of the
/// discordant pairs at least as uneven as the one seen, on either side:
/// `2 × Σ C(n, i) / 2^n` over `i` from 0 to the smaller count, with `n` the
/// sum of the two, and at most 1.
///
/// # Arguments
///
/// * `first_only` - The pairs that are right in the first output only.
/// * `second_only` - The pairs that are right in the second output only.
///
/// # Returns
///
/// The two-sided p-value; 1 without discordant pairs.
pub fn mcnemar_exact(first_only: usize, second_only: usize) -> f64 {
    let pairs = first_only + second_only;
    let smaller = first_only.min(second_only);
    // ln C(n, i) - n ln 2, from i = 0, in logarithms so that large counts
    // neither overflow nor vanish before they are summed.
    let mut term = -(pairs as f64) * std::f64::consts::LN_2;
    let mut tail = 0.0;
    for index in 0..=smaller {
        tail += term.exp();
        term += ((pairs - index) as f64).ln() - ((index + 1) as f64).ln();
    }
    (2.0 * tail).min(1.0)
}

/// The fold of a sentence in a cross-validation.
///
/// Sentences go to the folds in turn, by their index, so that the same file
/// gives the same folds everywhere, without random numbers.
///
/// # Arguments
///
/// * `index` - The index of the sentence in its file, from 0.
/// * `folds` - The number of folds.
///
/// # Returns
///
/// The fold, from 0: the sentence is in the test set of this fold, and in
/// the training set of every other.
pub fn fold_of(index: usize, folds: NonZeroUsize) -> usize {
    index % folds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentence::Bunsetsu;

    /// A morpheme that is its surface.
    struct Word(&'static str);

    impl Morpheme for Word {
        fn surface(&self) -> &str {
            self.0
        }

        fn feature(&self, _index: usize) -> Option<&str> {
            None
        }
    }

    /// A sentence of morphemes with the given surfaces, and bunsetsu given
    /// as `(number of morphemes, head)`.
    fn sentence(surfaces: &[&'static str], bunsetsu: &[(usize, Option<usize>)]) -> Sentence<Word> {
        let mut start = 0;
        let bunsetsu = bunsetsu
            .iter()
            .map(|&(length, head)| {
                let bunsetsu = Bunsetsu::new(start..start + length, head);
                start += length;
                bunsetsu
            })
            .collect();
        Sentence::new(
            surfaces.iter().map(|&surface| Word(surface)).collect(),
            bunsetsu,
        )
        .unwrap()
    }

    /// A count.
    fn count(correct: usize, total: usize) -> Count {
        Count { correct, total }
    }

    /// The four morphemes of most sentences here.
    const ABCD: &[&str] = &["a", "b", "c", "d"];

    /// The evaluation of one sentence.
    fn one(gold: Sentence<Word>, system: Sentence<Word>) -> Evaluation {
        evaluate(&[gold], &[system]).unwrap()
    }

    #[test]
    fn an_output_equal_to_the_answer_is_right_everywhere() {
        let heads = [(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, None)];
        let evaluation = one(sentence(ABCD, &heads), sentence(ABCD, &heads));
        assert_eq!(
            evaluation,
            Evaluation {
                sentences: 1,
                different: Vec::new(),
                without_bunsetsu: 0,
                chunking: Chunking {
                    correct: 4,
                    system: 4,
                    gold: 4,
                    sentences: count(1, 1),
                },
                dependency: Dependency {
                    level0: count(4, 4),
                    level1: count(3, 3),
                    level2: count(2, 2),
                    sentences: count(1, 1),
                },
                end_to_end: EndToEnd {
                    arcs: count(3, 3),
                    sentences: count(1, 1),
                },
            }
        );
        assert_eq!(evaluation.chunking.precision(), Some(1.0));
        assert_eq!(evaluation.chunking.recall(), Some(1.0));
        assert_eq!(evaluation.chunking.f_score(), Some(1.0));
    }

    #[test]
    fn a_bunsetsu_split_in_two() {
        // Answer: a | b c | d. Output: a | b | c | d.
        let gold = sentence(ABCD, &[(1, Some(1)), (2, Some(2)), (1, None)]);
        let system = sentence(ABCD, &[(1, Some(1)), (1, Some(2)), (1, Some(3)), (1, None)]);
        let evaluation = one(gold, system);
        // a and d are right; b starts where `b c` does and is shorter; c
        // starts where the answer has no bunsetsu.
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 2,
                system: 4,
                gold: 3,
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.chunking.precision(), Some(0.5));
        assert_eq!(evaluation.chunking.recall(), Some(2.0 / 3.0));
        let f_score = evaluation.chunking.f_score().unwrap();
        assert!((f_score - 4.0 / 7.0).abs() < 1e-12, "{f_score}");
        // Another segmentation: not in the dependency group.
        assert_eq!(evaluation.dependency, Dependency::default());
        // a → `b c`: the output has a, but its head is b. `b c` → d: the
        // output has no `b c`.
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(0, 2),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn as_many_bunsetsu_with_other_boundaries() {
        // Answer: a | b c | d. Output: a b | c | d, as many bunsetsu.
        let gold = sentence(ABCD, &[(1, Some(1)), (2, Some(2)), (1, None)]);
        let system = sentence(ABCD, &[(2, Some(1)), (1, Some(2)), (1, None)]);
        let evaluation = one(gold, system);
        // Only d is right.
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 1,
                system: 3,
                gold: 3,
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.dependency, Dependency::default());
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(0, 2),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn a_bunsetsu_of_another_length_is_wrong_even_with_the_right_head() {
        // Answer: a | b c | d, with a → d and `b c` → d. Output:
        // a | b | c | d, every bunsetsu → d. The output's b starts where
        // `b c` does and has its head, and is not `b c`.
        let gold = sentence(ABCD, &[(1, Some(2)), (2, Some(2)), (1, None)]);
        let system = sentence(ABCD, &[(1, Some(3)), (1, Some(3)), (1, Some(3)), (1, None)]);
        let evaluation = one(gold, system);
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(1, 2),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn two_bunsetsu_merged_into_one() {
        // Answer: a | b | c | d, with a → d. Output: a | b c | d, a → d.
        let gold = sentence(ABCD, &[(1, Some(3)), (1, Some(2)), (1, Some(3)), (1, None)]);
        let system = sentence(ABCD, &[(1, Some(2)), (2, Some(2)), (1, None)]);
        let evaluation = one(gold, system);
        // a and d are right; `b c` starts where b does and is longer; the
        // answer's c starts where the output has no bunsetsu.
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 2,
                system: 3,
                gold: 4,
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.dependency, Dependency::default());
        // a → d is right: the head has another index, and the same span.
        // b and c have no bunsetsu of their own in the output.
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(1, 3),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn a_wrong_head_with_the_same_segmentation() {
        // The head of b is d in the answer and c in the output.
        let gold = sentence(ABCD, &[(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, None)]);
        let system = sentence(ABCD, &[(1, Some(1)), (1, Some(2)), (1, Some(3)), (1, None)]);
        let evaluation = one(gold, system);
        assert_eq!(evaluation.chunking.correct, 4);
        assert_eq!(evaluation.chunking.sentences, count(1, 1));
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(3, 4),
                level1: count(2, 3),
                level2: count(1, 2),
                sentences: count(0, 1),
            }
        );
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(2, 3),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn the_last_and_the_last_but_one_bunsetsu_at_each_level() {
        let gold = || sentence(ABCD, &[(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, None)]);
        // The last-but-one bunsetsu, c, is wrong: it counts at levels 0 and
        // 1, and not at level 2.
        let system = sentence(ABCD, &[(1, Some(1)), (1, Some(3)), (1, None), (1, None)]);
        let evaluation = one(gold(), system);
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(3, 4),
                level1: count(2, 3),
                level2: count(2, 2),
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.end_to_end.arcs, count(2, 3));

        // The last bunsetsu, d, is wrong: it counts at level 0 only, and
        // the sentence is wrong.
        let system = sentence(
            ABCD,
            &[(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, Some(0))],
        );
        let evaluation = one(gold(), system);
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(3, 4),
                level1: count(3, 3),
                level2: count(2, 2),
                sentences: count(0, 1),
            }
        );
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(3, 3),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn a_sentence_of_one_bunsetsu() {
        // Right: it has no arc, and counts as a sentence.
        let evaluation = one(
            sentence(&["a", "b"], &[(2, None)]),
            sentence(&["a", "b"], &[(2, None)]),
        );
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 1,
                system: 1,
                gold: 1,
                sentences: count(1, 1),
            }
        );
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(1, 1),
                level1: count(0, 0),
                level2: count(0, 0),
                sentences: count(1, 1),
            }
        );
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(0, 0),
                sentences: count(1, 1),
            }
        );
        assert_eq!(evaluation.dependency.level1.ratio(), None);

        // Split in two by the output: the sentence is wrong, and still has
        // no arc.
        let evaluation = one(
            sentence(&["a", "b"], &[(2, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        );
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 0,
                system: 2,
                gold: 1,
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.chunking.f_score(), Some(0.0));
        assert_eq!(evaluation.dependency, Dependency::default());
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(0, 0),
                sentences: count(0, 1),
            }
        );
    }

    #[test]
    fn a_sentence_of_two_bunsetsu_is_at_level_1_and_not_at_level_2() {
        let evaluation = one(
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        );
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(2, 2),
                level1: count(1, 1),
                level2: count(0, 0),
                sentences: count(1, 1),
            }
        );
    }

    #[test]
    fn sentences_add_up() {
        let gold = [
            sentence(ABCD, &[(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        ];
        let system = [
            sentence(ABCD, &[(1, Some(1)), (1, Some(2)), (1, Some(3)), (1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        ];
        let evaluation = evaluate(&gold, &system).unwrap();
        assert_eq!(evaluation.sentences, 2);
        assert_eq!(
            evaluation.dependency,
            Dependency {
                level0: count(5, 6),
                level1: count(3, 4),
                level2: count(1, 2),
                sentences: count(1, 2),
            }
        );
        assert_eq!(
            evaluation.end_to_end,
            EndToEnd {
                arcs: count(3, 4),
                sentences: count(1, 2),
            }
        );
        assert_eq!(evaluation.chunking.sentences, count(2, 2));
    }

    #[test]
    fn a_sentence_with_different_morphemes_is_reported_and_not_counted() {
        let gold = [
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            // Another surface in the output.
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            // Another number of morphemes in the output.
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            // One more morpheme in the output, after the same ones.
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        ];
        let system = [
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            sentence(&["a", "x"], &[(1, Some(1)), (1, None)]),
            sentence(&["ab"], &[(1, None)]),
            sentence(&["a", "b", "c"], &[(1, Some(1)), (2, None)]),
        ];
        let evaluation = evaluate(&gold, &system).unwrap();
        assert_eq!(evaluation.sentences, 4);
        assert_eq!(evaluation.different, [2, 3, 4]);
        assert_eq!(evaluation.chunking.gold, 2);
        assert_eq!(evaluation.chunking.system, 2);
        assert_eq!(evaluation.chunking.sentences, count(1, 1));
        assert_eq!(evaluation.dependency.level0, count(2, 2));
        assert_eq!(evaluation.end_to_end.sentences, count(1, 1));
    }

    #[test]
    fn a_sentence_without_bunsetsu_in_the_answer_is_not_counted() {
        let gold = [sentence(&["a", "b"], &[])];
        let system = [sentence(&["a", "b"], &[(1, Some(1)), (1, None)])];
        let evaluation = evaluate(&gold, &system).unwrap();
        assert_eq!(
            evaluation,
            Evaluation {
                sentences: 1,
                without_bunsetsu: 1,
                ..Evaluation::default()
            }
        );

        // An output without bunsetsu, for an answer with some, is wrong.
        let evaluation = one(
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            sentence(&["a", "b"], &[]),
        );
        assert_eq!(
            evaluation.chunking,
            Chunking {
                correct: 0,
                system: 0,
                gold: 2,
                sentences: count(0, 1),
            }
        );
        assert_eq!(evaluation.chunking.precision(), None);
        assert_eq!(evaluation.chunking.f_score(), None);
        assert_eq!(evaluation.end_to_end.arcs, count(0, 1));
    }

    #[test]
    fn files_of_different_lengths_and_broken_sentences_are_errors() {
        let gold = [sentence(&["a"], &[(1, None)])];
        let system: [Sentence<Word>; 0] = [];
        let error = evaluate(&gold, &system).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the answer has 1 sentences and the output 0"
        );

        // A head that is no bunsetsu: `Sentence::check` refuses it.
        let mut broken = sentence(&["a"], &[(1, None)]);
        broken.bunsetsu[0].head = Some(5);
        let error = evaluate(&gold, std::slice::from_ref(&broken)).unwrap_err();
        assert!(matches!(error, Error::Structure(_)), "{error}");
        let error = evaluate(std::slice::from_ref(&broken), &gold).unwrap_err();
        assert!(matches!(error, Error::Structure(_)), "{error}");
    }

    #[test]
    fn two_outputs_are_paired_arc_by_arc_and_sentence_by_sentence() {
        let chain = [(1, Some(1)), (1, Some(2)), (1, Some(3)), (1, None)];
        let gold = [
            sentence(ABCD, &chain),
            sentence(ABCD, &chain),
            sentence(&["a"], &[(1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        ];
        let first = [
            // a right, b wrong, c right.
            sentence(ABCD, &[(1, Some(1)), (1, Some(3)), (1, Some(3)), (1, None)]),
            // a wrong, b wrong, c right.
            sentence(ABCD, &[(1, Some(2)), (1, Some(3)), (1, Some(3)), (1, None)]),
            sentence(&["a"], &[(1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
            sentence(&["a", "b"], &[(1, Some(1)), (1, None)]),
        ];
        let second = [
            // a wrong, b right, c right.
            sentence(ABCD, &[(1, Some(2)), (1, Some(2)), (1, Some(3)), (1, None)]),
            // a wrong, b wrong, c right.
            sentence(ABCD, &[(1, Some(3)), (1, Some(3)), (1, Some(3)), (1, None)]),
            sentence(&["a"], &[(1, None)]),
            // a wrong.
            sentence(&["a", "b"], &[(1, None), (1, None)]),
            // Other morphemes: left out of the pairs.
            sentence(&["a", "x"], &[(1, Some(1)), (1, None)]),
        ];
        let comparison = compare(&gold, &first, &second).unwrap();
        assert_eq!(
            comparison.arcs,
            Pairs {
                both: 2,
                first_only: 2,
                second_only: 1,
                neither: 2,
            }
        );
        assert_eq!(
            comparison.sentences,
            Pairs {
                both: 1,
                first_only: 1,
                second_only: 0,
                neither: 2,
            }
        );
        // Each output keeps its own evaluation: the last sentence counts
        // for the first one.
        assert_eq!(comparison.first.different, Vec::<usize>::new());
        assert_eq!(comparison.second.different, [5]);
        assert_eq!(comparison.first.end_to_end.arcs, count(5, 8));
        assert_eq!(comparison.second.end_to_end.arcs, count(3, 7));
        assert_eq!(comparison.first, evaluate(&gold, &first).unwrap());
        assert_eq!(comparison.second, evaluate(&gold, &second).unwrap());
        // 2 and 1 discordant arcs: 2 × (1 + 3) / 8.
        assert!((comparison.arcs.p_value() - 1.0).abs() < 1e-12);
        // 1 and 0 discordant sentences: 2 × 1 / 2.
        assert!((comparison.sentences.p_value() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mcnemar_exact_gives_the_values_computed_by_hand() {
        let close = |value: f64, expected: f64| (value - expected).abs() < 1e-12;
        assert_eq!(mcnemar_exact(0, 0), 1.0);
        assert!(close(mcnemar_exact(1, 0), 1.0));
        assert_eq!(mcnemar_exact(3, 3), 1.0);
        // 2 × 1 / 2^5.
        assert!(close(mcnemar_exact(5, 0), 0.0625));
        assert!(close(mcnemar_exact(0, 5), 0.0625));
        // 2 × (1 + 12 + 66) / 2^12.
        assert!(close(mcnemar_exact(10, 2), 158.0 / 4096.0));
        assert!(close(mcnemar_exact(2, 10), 158.0 / 4096.0));
        // 2 × (1 + 8) / 2^8.
        assert!(close(mcnemar_exact(7, 1), 18.0 / 256.0));
    }

    #[test]
    fn mcnemar_exact_holds_for_large_counts() {
        // An even split of many pairs: no evidence of a difference.
        assert_eq!(mcnemar_exact(20_000, 20_000), 1.0);
        // A very uneven split: tiny, and still a number.
        let p_value = mcnemar_exact(30_000, 10_000);
        assert!((0.0..1e-300).contains(&p_value), "{p_value}");
        // 5,100 against 4,900: about two standard deviations, so the
        // p-value is near 0.046 (the normal approximation gives 0.0455).
        let p_value = mcnemar_exact(5_100, 4_900);
        assert!((0.044..0.049).contains(&p_value), "{p_value}");
    }

    #[test]
    fn folds_take_the_sentences_in_turn() {
        let three = NonZeroUsize::new(3).unwrap();
        let folds: Vec<usize> = (0..7).map(|index| fold_of(index, three)).collect();
        assert_eq!(folds, [0, 1, 2, 0, 1, 2, 0]);
        let one = NonZeroUsize::new(1).unwrap();
        assert_eq!(fold_of(41, one), 0);
    }

    #[test]
    fn an_evaluation_is_written_line_by_line() {
        let evaluation = Evaluation {
            sentences: 14,
            different: (1..=12).collect(),
            without_bunsetsu: 1,
            chunking: Chunking {
                correct: 9,
                system: 10,
                gold: 12,
                sentences: count(0, 1),
            },
            dependency: Dependency {
                level0: count(3, 4),
                level1: count(2, 3),
                level2: count(1, 2),
                sentences: count(0, 0),
            },
            end_to_end: EndToEnd {
                arcs: count(1, 8),
                sentences: count(0, 1),
            },
        };
        assert_eq!(
            evaluation.to_string(),
            "\
sentences: 14
left out, different morphemes: 12 (sentences 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 and 2 more)
not counted, no bunsetsu: 1
chunking precision: 90.00% (9/10)
chunking recall: 75.00% (9/12)
chunking F: 81.82%
chunking sentences: 0.00% (0/1)
dependency level 0: 75.00% (3/4)
dependency level 1: 66.67% (2/3)
dependency level 2: 50.00% (1/2)
dependency sentences: n/a (0/0)
end to end: 12.50% (1/8)
end to end sentences: 0.00% (0/1)"
        );
        assert_eq!(
            Evaluation::default().to_string().lines().nth(1),
            Some("left out, different morphemes: 0")
        );
    }

    #[test]
    fn a_comparison_is_written_with_its_pairs() {
        let comparison = Comparison {
            arcs: Pairs {
                both: 80,
                first_only: 10,
                second_only: 2,
                neither: 8,
            },
            sentences: Pairs {
                both: 5,
                first_only: 5,
                second_only: 0,
                neither: 1,
            },
            ..Comparison::default()
        };
        let text = comparison.to_string();
        assert!(text.starts_with("first output\nsentences: 0\n"), "{text}");
        assert!(text.contains("\n\nsecond output\nsentences: 0\n"), "{text}");
        assert!(
            text.ends_with(
                "paired arcs: both 80, first only 10, second only 2, neither 8, McNemar p = 0.0386\n\
                 paired sentences: both 5, first only 5, second only 0, neither 1, McNemar p = 0.0625"
            ),
            "{text}"
        );
    }
}

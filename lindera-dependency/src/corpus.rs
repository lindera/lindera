//! Training data from a dependency corpus.
//!
//! The parser is trained on bunsetsu dependencies, on the morphemes of the
//! dictionary it will run with. This module makes such data from UD
//! Japanese-GSD (and UD Japanese-PUD), which have word dependencies and the
//! bunsetsu boundaries, in two steps:
//!
//! 1. [`to_bunsetsu`] turns the word dependencies of a sentence into
//!    bunsetsu dependencies, with coordination in the form of the Kyoto
//!    corpus and CaboCha.
//! 2. [`project()`] lays the bunsetsu over the morphemes that a tokenizer
//!    gives for the text of the sentence, merging the bunsetsu whose
//!    boundary falls inside a morpheme.
//!
//! [`convert()`] does both for a whole file, writes the lattice format and
//! counts what it did in a [`Report`], with how often the head word of a
//! [`Schema`] is the content head of the corpus. Nothing here depends on a
//! tokenizer:
//! the caller gives the morphemes. The `lindera dependency convert` command
//! tokenizes with Lindera.
//!
//! The corpora are not part of this repository. See the README of the crate
//! for where to get them and for their licenses.
//!
//! # Examples
//!
//! ```
//! use lindera_dependency::Schema;
//! use lindera_dependency::corpus::{self, DictionaryMorpheme};
//!
//! // 彼は | 本を | 読んだ, in the columns the conversion reads.
//! let conllu = "\
//! ## text = 彼は本を読んだ
//! 1\t彼\t_\t_\t_\t_\t5\tnsubj\t_\tBunsetuBILabel=B
//! 2\tは\t_\t_\t_\t_\t1\tcase\t_\tBunsetuBILabel=I
//! 3\t本\t_\t_\t_\t_\t5\tobj\t_\tBunsetuBILabel=B
//! 4\tを\t_\t_\t_\t_\t3\tcase\t_\tBunsetuBILabel=I
//! 5\t読ん\t_\t_\t_\t_\t0\troot\t_\tBunsetuBILabel=B
//! 6\tだ\t_\t_\t_\t_\t5\taux\t_\tBunsetuBILabel=I
//! ";
//!
//! // A tokenizer that gives one morpheme per character, without features.
//! let tokenize = |text: &str| {
//!     Ok(text
//!         .chars()
//!         .map(|character| DictionaryMorpheme::new(character.to_string(), vec!["*".to_string()]))
//!         .collect::<Vec<_>>())
//! };
//!
//! let mut output = Vec::new();
//! let report = corpus::convert(conllu.as_bytes(), &mut output, &Schema::ipadic(), tokenize)?;
//! assert_eq!(report.kept, 1);
//! assert_eq!(
//!     String::from_utf8(output).unwrap(),
//!     "\
//! * 0 2D \n彼\t*\nは\t*\n\
//! * 1 2D \n本\t*\nを\t*\n\
//! * 2 -1D \n読\t*\nん\t*\nだ\t*\n\
//! EOS\n"
//! );
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

pub mod conllu;
pub mod convert;
pub mod project;

use std::collections::BTreeMap;
use std::fmt;
use std::io::{BufRead, Write};

pub use conllu::{ConlluReader, UdSentence, UdWord};
pub use convert::{BunsetsuTree, UdBunsetsu, to_bunsetsu};
pub use project::{Projection, project};

// The morpheme a tokenizer gives, which the command needs without the
// training code as well: it is in the default build.
pub use crate::morpheme::DictionaryMorpheme;

use crate::error::{Error, Result};
use crate::lattice;
use crate::morpheme::Morpheme;
use crate::schema::Schema;
use crate::sentence::ArcType;

/// The number of kinds of disagreement on the head word a [`Report`]
/// writes: the most frequent ones.
const DISAGREEMENTS_SHOWN: usize = 10;

/// Why a sentence of a corpus is left out of the training data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DropCause {
    /// The bunsetsu cannot be made: the first word starts no bunsetsu, or a
    /// bunsetsu has no word whose head is outside it.
    Bunsetsu,
    /// There is no root, more than one, or the root is not the last
    /// bunsetsu.
    Root,
    /// A bunsetsu depends on one to its left, or on itself.
    HeadToTheLeft,
    /// Two arcs cross.
    CrossingArcs,
    /// The morphemes of the tokenizer do not spell the words of the
    /// sentence.
    Text,
}

impl DropCause {
    /// Every cause, in the order of the report.
    const ALL: [DropCause; 5] = [
        DropCause::Bunsetsu,
        DropCause::Root,
        DropCause::HeadToTheLeft,
        DropCause::CrossingArcs,
        DropCause::Text,
    ];

    /// The name of the cause in the report.
    ///
    /// # Returns
    ///
    /// The name.
    fn name(self) -> &'static str {
        match self {
            DropCause::Bunsetsu => "bunsetsu that cannot be made",
            DropCause::Root => "root",
            DropCause::HeadToTheLeft => "arc to the left",
            DropCause::CrossingArcs => "crossing arcs",
            DropCause::Text => "text that does not match",
        }
    }
}

impl fmt::Display for DropCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A way the head word of a schema is not the content head of the corpus:
/// the parts of speech of the two morphemes, and their order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeadDisagreement {
    /// The part of speech of the head word by the schema, as
    /// [`Schema::part_of_speech_of`] gives it.
    pub schema: String,
    /// The part of speech of the morpheme with the content head of the
    /// corpus.
    pub corpus: String,
    /// Whether the head word by the schema is to the right of the content
    /// head of the corpus, or to its left.
    pub schema_is_right: bool,
}

/// What the conversion of a file did.
///
/// [`fmt::Display`] writes it as the lines of a report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The sentences read.
    pub sentences: usize,
    /// The sentences written.
    pub kept: usize,
    /// The sentences left out.
    pub dropped: usize,
    /// The sentences left out by cause, in the order of [`DropCause`]. A
    /// sentence with several causes counts for each, so the sum can be
    /// larger than `dropped`.
    pub causes: [usize; 5],
    /// The bunsetsu with more than one word whose head is outside the
    /// bunsetsu, in the sentences that are a tree before the projection.
    pub ambiguous_heads: usize,
    /// The arcs of the sentences that are a tree before the projection: one
    /// per bunsetsu but the root.
    pub arcs: usize,
    /// Those of the arcs that have the type `P`.
    pub parallel_arcs: usize,
    /// The bunsetsu boundaries inside the sentences written, before the
    /// projection.
    pub boundaries: usize,
    /// Those of the boundaries that are morpheme boundaries.
    pub boundaries_fit: usize,
    /// The sentences written in which bunsetsu were merged.
    pub merged_sentences: usize,
    /// The bunsetsu lost by merging, in the sentences written.
    pub merged: usize,
    /// The bunsetsu written.
    pub bunsetsu: usize,
    /// The arcs of type `P` written. A merged bunsetsu takes the arc of the
    /// last one merged, so fewer remain than `parallel_arcs`.
    pub parallel_arcs_written: usize,
    /// The morphemes written.
    pub morphemes: usize,
    /// The bunsetsu written whose content head the corpus gives
    /// (`BunsetuPositionType`).
    pub heads: usize,
    /// Those whose head word by the schema is the morpheme with that
    /// content head.
    pub heads_agree: usize,
    /// The others, by kind.
    pub head_disagreements: BTreeMap<HeadDisagreement, usize>,
}

impl Report {
    /// Counts a sentence read.
    fn count_read(&mut self) {
        self.sentences += 1;
    }

    /// Counts a sentence left out.
    ///
    /// # Arguments
    ///
    /// * `causes` - Its causes, each once.
    fn count_dropped(&mut self, causes: &[DropCause]) {
        self.dropped += 1;
        for cause in causes {
            if let Some(index) = DropCause::ALL.iter().position(|other| other == cause) {
                self.causes[index] += 1;
            }
        }
    }

    /// Counts a sentence that is a tree before the projection.
    ///
    /// # Arguments
    ///
    /// * `tree` - Its bunsetsu.
    fn count_converted(&mut self, tree: &BunsetsuTree) {
        self.ambiguous_heads += tree.ambiguous_heads;
        for bunsetsu in &tree.bunsetsu {
            if bunsetsu.head.is_some() {
                self.arcs += 1;
                if bunsetsu.arc_type == ArcType::Parallel {
                    self.parallel_arcs += 1;
                }
            }
        }
    }

    /// Counts a sentence written.
    ///
    /// # Arguments
    ///
    /// * `projection` - The sentence.
    /// * `schema` - The schema whose head word is compared with the content
    ///   head of the corpus.
    fn count_kept<M: Morpheme>(&mut self, projection: &Projection<M>, schema: &Schema) {
        self.kept += 1;
        self.boundaries += projection.boundaries;
        self.boundaries_fit += projection.boundaries_fit;
        self.merged += projection.merged;
        if projection.merged > 0 {
            self.merged_sentences += 1;
        }
        self.bunsetsu += projection.sentence.bunsetsu.len();
        self.parallel_arcs_written += projection
            .sentence
            .bunsetsu
            .iter()
            .filter(|bunsetsu| bunsetsu.head.is_some() && bunsetsu.arc_type == ArcType::Parallel)
            .count();
        self.morphemes += projection.sentence.morphemes.len();

        let sentence = &projection.sentence;
        for (bunsetsu, content_head) in sentence.bunsetsu.iter().zip(&projection.content_heads) {
            let morphemes = &sentence.morphemes[bunsetsu.morphemes.clone()];
            let (Some(content_head), Some((head, _))) =
                (*content_head, schema.positions(morphemes))
            else {
                continue;
            };
            self.heads += 1;
            if head == content_head {
                self.heads_agree += 1;
                continue;
            }
            let disagreement = HeadDisagreement {
                schema: schema.part_of_speech_of(&morphemes[head]),
                corpus: schema.part_of_speech_of(&morphemes[content_head]),
                schema_is_right: head > content_head,
            };
            *self.head_disagreements.entry(disagreement).or_default() += 1;
        }
    }

    /// The number of sentences left out for a cause.
    ///
    /// # Arguments
    ///
    /// * `cause` - The cause.
    ///
    /// # Returns
    ///
    /// The number of sentences that have the cause, alone or with others.
    pub fn dropped_for(&self, cause: DropCause) -> usize {
        DropCause::ALL
            .iter()
            .position(|other| *other == cause)
            .map_or(0, |index| self.causes[index])
    }
}

/// A count with its share of a whole, as `count of whole (share%)`.
struct Share(usize, usize);

impl fmt::Display for Share {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Share(count, whole) = *self;
        if whole == 0 {
            write!(f, "{count} of {whole}")
        } else {
            write!(
                f,
                "{count} of {whole} ({:.2}%)",
                100.0 * count as f64 / whole as f64
            )
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "sentences kept: {}", Share(self.kept, self.sentences))?;
        writeln!(f, "sentences dropped: {}", self.dropped)?;
        for (cause, count) in DropCause::ALL.iter().zip(self.causes) {
            writeln!(f, "  {cause}: {count}")?;
        }
        writeln!(
            f,
            "bunsetsu with more than one word that leaves it: {}",
            self.ambiguous_heads
        )?;
        writeln!(
            f,
            "arcs of type P: {}",
            Share(self.parallel_arcs, self.arcs)
        )?;
        writeln!(
            f,
            "bunsetsu boundaries on a morpheme boundary: {}",
            Share(self.boundaries_fit, self.boundaries)
        )?;
        writeln!(
            f,
            "sentences with merged bunsetsu: {}",
            Share(self.merged_sentences, self.kept)
        )?;
        writeln!(
            f,
            "bunsetsu merged: {}",
            Share(self.merged, self.bunsetsu + self.merged)
        )?;
        writeln!(f, "bunsetsu written: {}", self.bunsetsu)?;
        writeln!(
            f,
            "arcs of type P written: {}",
            Share(
                self.parallel_arcs_written,
                self.bunsetsu.saturating_sub(self.kept)
            )
        )?;
        writeln!(f, "morphemes written: {}", self.morphemes)?;
        write!(
            f,
            "head words on the content head of the corpus: {}",
            Share(self.heads_agree, self.heads)
        )?;
        // The most frequent kinds first, and the same order every time.
        let mut disagreements: Vec<(&HeadDisagreement, &usize)> =
            self.head_disagreements.iter().collect();
        disagreements.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (disagreement, count) in disagreements.into_iter().take(DISAGREEMENTS_SHOWN) {
            let side = if disagreement.schema_is_right {
                "right"
            } else {
                "left"
            };
            write!(
                f,
                "\n  {count}: {} is the head word, to the {side} of {}",
                disagreement.schema, disagreement.corpus
            )?;
        }
        Ok(())
    }
}

/// The error type a tokenizer given to [`convert()`] returns.
pub type TokenizerError = Box<dyn std::error::Error + Send + Sync>;

/// Converts a CoNLL-U file to training data in the lattice format.
///
/// Each sentence goes through [`to_bunsetsu`] and, with the morphemes that
/// `tokenize` gives for its text, through [`project()`]. The sentences that
/// remain are written with bunsetsu lines that hold the head and the arc
/// type only, without comments: the positions of the head word and of the
/// function word are not written, since what reads the data computes them
/// from its schema.
///
/// For the bunsetsu whose content head the corpus gives
/// (`BunsetuPositionType=SEM_HEAD`, or `ROOT`), the report counts how often
/// the head word of `schema` is the morpheme with the first character of
/// that word. The function word is not compared: the corpus has none that
/// is the one of a schema.
///
/// # Arguments
///
/// * `reader` - The corpus, in CoNLL-U with `BunsetuBILabel` in the last
///   column, as UD Japanese-GSD and UD Japanese-PUD have it.
/// * `writer` - Where the lattice format is written.
/// * `schema` - The schema of the dictionary of the morphemes, whose head
///   word the report measures. It does not change what is written.
/// * `tokenize` - Gives the morphemes of a text. It has to be the tokenizer,
///   with the dictionary and the settings, the parser will run with.
///
/// # Returns
///
/// The report, or an error: [`Error::Conllu`] for a line that cannot be
/// read and for a sentence without a `# text` comment, [`Error::Tokenizer`]
/// when `tokenize` fails, [`Error::Structure`] for a morpheme the lattice
/// format cannot hold, [`Error::Io`] when reading or writing fails.
pub fn convert<R, W, M, F>(
    reader: R,
    writer: &mut W,
    schema: &Schema,
    mut tokenize: F,
) -> Result<Report>
where
    R: BufRead,
    W: Write,
    M: Morpheme,
    F: FnMut(&str) -> std::result::Result<Vec<M>, TokenizerError>,
{
    let mut report = Report::default();
    for sentence in ConlluReader::new(reader) {
        let sentence = sentence?;
        report.count_read();
        let tree = match to_bunsetsu(&sentence) {
            Ok(tree) => tree,
            Err(causes) => {
                report.count_dropped(&causes);
                continue;
            }
        };
        report.count_converted(&tree);
        let text = sentence
            .text()
            .ok_or_else(|| Error::conllu(sentence.line, "the sentence has no `# text` comment"))?;
        let morphemes = tokenize(text).map_err(Error::Tokenizer)?;
        match project(&sentence, &tree, morphemes) {
            Ok(projection) => {
                lattice::write(writer, &projection.sentence)?;
                report.count_kept(&projection, schema);
            }
            Err(causes) => report.count_dropped(&causes),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A UD sentence of words given as `(form, head, relation, whether the
    /// word starts a bunsetsu)`, with the forms as its text.
    pub(super) fn ud_sentence(words: &[(&str, usize, &str, bool)]) -> UdSentence {
        let text: String = words.iter().map(|word| word.0).collect();
        UdSentence {
            line: 1,
            comments: vec![format!("# text = {text}")],
            words: words
                .iter()
                .map(|&(form, head, deprel, starts_bunsetsu)| UdWord {
                    form: form.to_string(),
                    head,
                    deprel: deprel.to_string(),
                    starts_bunsetsu,
                    content_head: false,
                })
                .collect(),
        }
    }

    /// A sentence with some of its words marked as content heads.
    pub(super) fn with_content_heads(mut sentence: UdSentence, words: &[usize]) -> UdSentence {
        for &word in words {
            sentence.words[word].content_head = true;
        }
        sentence
    }

    /// Morphemes with the given surfaces and the one feature column `*`.
    pub(super) fn morphemes(surfaces: &[&str]) -> Vec<DictionaryMorpheme> {
        surfaces
            .iter()
            .map(|surface| DictionaryMorpheme::new(surface.to_string(), vec!["*".to_string()]))
            .collect()
    }

    /// The CoNLL-U text of sentences of words given as for [`ud_sentence`].
    fn conllu_text(sentences: &[&[(&str, usize, &str, bool)]]) -> String {
        let sentences: Vec<Sentence<'_>> = sentences.iter().map(|words| (*words, EMPTY)).collect();
        conllu_text_with_content_heads(&sentences)
    }

    /// The words of a sentence as for [`ud_sentence`], and the indices of
    /// those that are content heads.
    type Sentence<'a> = (&'a [(&'a str, usize, &'a str, bool)], &'a [usize]);

    /// No content head.
    const EMPTY: &[usize] = &[];

    /// The CoNLL-U text of sentences, with `BunsetuPositionType=SEM_HEAD`
    /// on the content heads and `CONT` on the other words when a sentence
    /// has content heads, and without the key when it has none.
    fn conllu_text_with_content_heads(sentences: &[Sentence<'_>]) -> String {
        let mut text = String::new();
        for (words, content_heads) in sentences {
            let forms: String = words.iter().map(|word| word.0).collect();
            text.push_str(&format!("# text = {forms}\n"));
            for (index, (form, head, deprel, starts_bunsetsu)) in words.iter().enumerate() {
                let label = if *starts_bunsetsu { "B" } else { "I" };
                let position = if content_heads.is_empty() {
                    ""
                } else if content_heads.contains(&index) {
                    "|BunsetuPositionType=SEM_HEAD"
                } else {
                    "|BunsetuPositionType=CONT"
                };
                text.push_str(&format!(
                    "{}\t{form}\t_\t_\t_\t_\t{head}\t{deprel}\t_\tBunsetuBILabel={label}{position}\n",
                    index + 1
                ));
            }
            text.push('\n');
        }
        text
    }

    /// A tokenizer that splits a text at the given surfaces, in their
    /// order, and gives the rest character by character.
    fn tokenizer(
        surfaces: &'static [&'static str],
    ) -> impl FnMut(&str) -> std::result::Result<Vec<DictionaryMorpheme>, TokenizerError> {
        move |text: &str| {
            let mut result = Vec::new();
            let mut rest = text;
            'text: while !rest.is_empty() {
                for surface in surfaces {
                    if let Some(after) = rest.strip_prefix(surface) {
                        result.push(surface.to_string());
                        rest = after;
                        continue 'text;
                    }
                }
                let length = rest.chars().next().map_or(0, char::len_utf8);
                result.push(rest[..length].to_string());
                rest = &rest[length..];
            }
            Ok(result
                .into_iter()
                .map(|surface| DictionaryMorpheme::new(surface, vec!["*".to_string()]))
                .collect())
        }
    }

    /// One valid sentence, 彼は | 本を | 読む.
    const VALID: &[(&str, usize, &str, bool)] = &[
        ("彼", 5, "nsubj", true),
        ("は", 1, "case", false),
        ("本", 5, "obj", true),
        ("を", 3, "case", false),
        ("読む", 0, "root", true),
    ];

    #[test]
    fn converts_a_file_and_writes_head_only_bunsetsu_lines() {
        let input = conllu_text(&[VALID]);
        let mut output = Vec::new();
        let report = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            tokenizer(&["読む"]),
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "* 0 2D \n彼\t*\nは\t*\n* 1 2D \n本\t*\nを\t*\n* 2 -1D \n読む\t*\nEOS\n"
        );
        assert_eq!(
            report,
            Report {
                sentences: 1,
                kept: 1,
                arcs: 2,
                boundaries: 2,
                boundaries_fit: 2,
                bunsetsu: 3,
                morphemes: 5,
                ..Report::default()
            }
        );
    }

    #[test]
    fn writes_the_arc_type_and_reads_back_as_a_tree() {
        // a | b | 読む with b a conjunct of a.
        let input = conllu_text(&[&[
            ("a", 3, "obj", true),
            ("b", 1, "conj", true),
            ("読む", 0, "root", true),
        ]]);
        let mut output = Vec::new();
        let report = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            tokenizer(&["読む"]),
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(
            output,
            "* 0 1P \na\t*\n* 1 2D \nb\t*\n* 2 -1D \n読む\t*\nEOS\n"
        );
        assert_eq!(
            (
                report.arcs,
                report.parallel_arcs,
                report.parallel_arcs_written
            ),
            (2, 1, 1)
        );

        let sentences = lattice::parse(&output).unwrap();
        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].violations().is_empty());
        assert_eq!(sentences[0].bunsetsu[0].arc_type, ArcType::Parallel);
    }

    #[test]
    fn quotes_a_feature_column_with_a_comma() {
        let input = conllu_text(&[&[("a", 2, "dep", true), (",", 0, "root", true)]]);
        let tokenize = |text: &str| {
            Ok(text
                .chars()
                .map(|character| {
                    let surface = character.to_string();
                    DictionaryMorpheme::new(surface.clone(), vec!["記号".to_string(), surface])
                })
                .collect::<Vec<_>>())
        };
        let mut output = Vec::new();
        convert(input.as_bytes(), &mut output, &Schema::ipadic(), tokenize).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output, "* 0 1D \na\t記号,a\n* 1 -1D \n,\t記号,\",\"\nEOS\n");
        // Read back, the comma is one column.
        let sentences = lattice::parse(&output).unwrap();
        assert_eq!(sentences[0].morphemes[1].feature(1), Some(","));
        assert_eq!(sentences[0].morphemes[1].feature(2), None);
    }

    #[test]
    fn counts_what_it_keeps_merges_and_drops() {
        let input = conllu_text(&[
            // Kept, with その | 後 merged by the morpheme その後.
            &[
                ("その", 4, "det", true),
                ("後", 3, "obl", true),
                ("本", 4, "obj", true),
                ("読む", 0, "root", true),
            ],
            // Kept, with a conjunct and a bunsetsu two words leave.
            &[
                ("（", 4, "punct", true),
                ("a", 4, "obj", false),
                ("b", 2, "conj", true),
                ("読む", 0, "root", true),
            ],
            // Dropped: the root is not the last bunsetsu, and an arc goes to
            // the left.
            &[("読む", 0, "root", true), ("彼", 1, "nsubj", true)],
            // Dropped: crossing arcs.
            &[
                ("a", 3, "dep", true),
                ("b", 4, "dep", true),
                ("c", 4, "dep", true),
                ("d", 0, "root", true),
            ],
            // Dropped: the first word starts no bunsetsu.
            &[("彼", 2, "nsubj", false), ("読む", 0, "root", true)],
            // Dropped: the tokenizer changes the text (see below).
            &[("x", 2, "dep", true), ("読む", 0, "root", true)],
        ]);
        let mut split = tokenizer(&["その後", "読む"]);
        let tokenize = move |text: &str| {
            let mut morphemes = split(text)?;
            for morpheme in &mut morphemes {
                if morpheme.surface == "x" {
                    morpheme.surface = "y".to_string();
                }
            }
            Ok(morphemes)
        };
        let mut output = Vec::new();
        let report = convert(input.as_bytes(), &mut output, &Schema::ipadic(), tokenize).unwrap();
        assert_eq!(
            report,
            Report {
                sentences: 6,
                kept: 2,
                dropped: 4,
                // Bunsetsu, Root, HeadToTheLeft, CrossingArcs, Text.
                causes: [1, 1, 1, 1, 1],
                ambiguous_heads: 1,
                // 3 + 2 arcs of the kept sentences and 1 of the sentence
                // dropped by the projection.
                arcs: 6,
                parallel_arcs: 1,
                boundaries: 5,
                boundaries_fit: 4,
                merged_sentences: 1,
                merged: 1,
                bunsetsu: 6,
                parallel_arcs_written: 1,
                morphemes: 7,
                ..Report::default()
            }
        );
        assert_eq!(report.dropped_for(DropCause::Root), 1);
        assert_eq!(report.dropped_for(DropCause::Text), 1);
        assert_eq!(
            String::from_utf8(output).unwrap().matches("EOS\n").count(),
            2
        );
    }

    /// A tokenizer that splits 利用した本を読む into morphemes with the
    /// features of IPADIC.
    fn ipadic_tokenizer(_: &str) -> std::result::Result<Vec<DictionaryMorpheme>, TokenizerError> {
        Ok([
            ("利用", "名詞,サ変接続,*,*,*,*,利用"),
            ("し", "動詞,自立,*,*,サ変・スル,連用形,する"),
            ("た", "助動詞,*,*,*,特殊・タ,基本形,た"),
            ("本", "名詞,一般,*,*,*,*,本"),
            ("を", "助詞,格助詞,一般,*,*,*,を"),
            ("読む", "動詞,自立,*,*,五段・マ行,基本形,読む"),
        ]
        .into_iter()
        .map(|(surface, features)| {
            DictionaryMorpheme::new(
                surface.to_string(),
                features.split(',').map(str::to_string).collect(),
            )
        })
        .collect())
    }

    /// 利用 し た | 本 を | 読む.
    const SURU: &[(&str, usize, &str, bool)] = &[
        ("利用", 4, "acl", true),
        ("し", 1, "aux", false),
        ("た", 1, "aux", false),
        ("本", 6, "obj", true),
        ("を", 4, "case", false),
        ("読む", 0, "root", true),
    ];

    #[test]
    fn counts_the_head_words_on_the_content_head_of_the_corpus() {
        // The corpus has 利用, 本 and 読む as content heads, as the schema
        // of IPADIC does.
        let input = conllu_text_with_content_heads(&[(SURU, &[0, 3, 5])]);
        let mut output = Vec::new();
        let report = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            ipadic_tokenizer,
        )
        .unwrap();
        assert_eq!((report.heads, report.heads_agree), (3, 3));
        assert!(report.head_disagreements.is_empty());
        let with_ipadic = output;

        // CaboCha's rule alone has し as the head word of the first
        // bunsetsu; and a corpus with を as a content head disagrees to
        // the other side.
        let mut cabocha = Schema::ipadic();
        cabocha.function_words.truncate(6);
        let input = conllu_text_with_content_heads(&[(SURU, &[0, 4, 5])]);
        let mut output = Vec::new();
        let report = convert(input.as_bytes(), &mut output, &cabocha, ipadic_tokenizer).unwrap();
        assert_eq!((report.heads, report.heads_agree), (3, 1));
        let disagreements: Vec<(HeadDisagreement, usize)> =
            report.head_disagreements.into_iter().collect();
        assert_eq!(
            disagreements,
            [
                (
                    HeadDisagreement {
                        schema: "動詞-自立".to_string(),
                        corpus: "名詞-サ変接続".to_string(),
                        schema_is_right: true,
                    },
                    1
                ),
                (
                    HeadDisagreement {
                        schema: "名詞-一般".to_string(),
                        corpus: "助詞-格助詞-一般".to_string(),
                        schema_is_right: false,
                    },
                    1
                ),
            ]
        );
        // The schema and the content heads change the report, not what is
        // written.
        assert_eq!(output, with_ipadic);
        assert!(!String::from_utf8(output).unwrap().contains('/'));
    }

    #[test]
    fn a_bunsetsu_without_one_content_head_is_not_measured() {
        // No word of the second bunsetsu is marked, and two of the first.
        let input = conllu_text_with_content_heads(&[(SURU, &[0, 1, 5])]);
        let mut output = Vec::new();
        let report = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            ipadic_tokenizer,
        )
        .unwrap();
        assert_eq!(
            (report.bunsetsu, report.heads, report.heads_agree),
            (3, 1, 1)
        );
    }

    #[test]
    fn the_report_shows_the_most_frequent_disagreements() {
        let disagreement = |schema: &str, corpus: &str, schema_is_right| HeadDisagreement {
            schema: schema.to_string(),
            corpus: corpus.to_string(),
            schema_is_right,
        };
        let mut report = Report {
            heads: 200,
            heads_agree: 150,
            ..Report::default()
        };
        // 12 kinds: `k00` once, `k01` twice, and so on; and two kinds with
        // the same count.
        for index in 0..12 {
            report.head_disagreements.insert(
                disagreement(&format!("k{index:02}"), "名詞", true),
                index + 1,
            );
        }
        report
            .head_disagreements
            .insert(disagreement("k11", "名詞", false), 12);
        let text = report.to_string();
        let lines: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with("head words"))
            .collect();
        assert_eq!(
            lines,
            [
                "head words on the content head of the corpus: 150 of 200 (75.00%)",
                // The same count: to the left before to the right.
                "  12: k11 is the head word, to the left of 名詞",
                "  12: k11 is the head word, to the right of 名詞",
                "  11: k10 is the head word, to the right of 名詞",
                "  10: k09 is the head word, to the right of 名詞",
                "  9: k08 is the head word, to the right of 名詞",
                "  8: k07 is the head word, to the right of 名詞",
                "  7: k06 is the head word, to the right of 名詞",
                "  6: k05 is the head word, to the right of 名詞",
                "  5: k04 is the head word, to the right of 名詞",
                "  4: k03 is the head word, to the right of 名詞",
            ]
        );
        assert!(!text.ends_with('\n'));
    }

    #[test]
    fn the_report_is_written_line_by_line() {
        let report = Report {
            sentences: 8,
            kept: 6,
            dropped: 2,
            causes: [0, 1, 1, 1, 0],
            ambiguous_heads: 1,
            arcs: 300,
            parallel_arcs: 3,
            boundaries: 200,
            boundaries_fit: 199,
            merged_sentences: 1,
            merged: 1,
            bunsetsu: 205,
            parallel_arcs_written: 2,
            morphemes: 500,
            ..Report::default()
        };
        assert_eq!(
            report.to_string(),
            "\
sentences kept: 6 of 8 (75.00%)
sentences dropped: 2
  bunsetsu that cannot be made: 0
  root: 1
  arc to the left: 1
  crossing arcs: 1
  text that does not match: 0
bunsetsu with more than one word that leaves it: 1
arcs of type P: 3 of 300 (1.00%)
bunsetsu boundaries on a morpheme boundary: 199 of 200 (99.50%)
sentences with merged bunsetsu: 1 of 6 (16.67%)
bunsetsu merged: 1 of 206 (0.49%)
bunsetsu written: 205
arcs of type P written: 2 of 199 (1.01%)
morphemes written: 500
head words on the content head of the corpus: 0 of 0"
        );
        assert_eq!(
            Report::default().to_string().lines().next(),
            Some("sentences kept: 0 of 0")
        );
    }

    #[test]
    fn fails_on_a_sentence_without_text_and_on_a_failing_tokenizer() {
        let input = "1\t彼\t_\t_\t_\t_\t0\troot\t_\tBunsetuBILabel=B\n";
        let mut output = Vec::new();
        let error = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            tokenizer(&[]),
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "line 1: the sentence has no `# text` comment"
        );

        let input = conllu_text(&[VALID]);
        let failing = |_: &str| -> std::result::Result<Vec<DictionaryMorpheme>, TokenizerError> {
            Err("no dictionary".into())
        };
        let error = convert(input.as_bytes(), &mut output, &Schema::ipadic(), failing).unwrap_err();
        assert_eq!(error.to_string(), "tokenizer: no dictionary");
        assert!(output.is_empty());
    }

    #[test]
    fn a_dropped_sentence_needs_no_text() {
        // The conversion drops the sentence before its text is looked at.
        let input = "\
1\t読む\t_\t_\t_\t_\t0\troot\t_\tBunsetuBILabel=B
2\t彼\t_\t_\t_\t_\t1\tnsubj\t_\tBunsetuBILabel=B
";
        let mut output = Vec::new();
        let report = convert(
            input.as_bytes(),
            &mut output,
            &Schema::ipadic(),
            tokenizer(&[]),
        )
        .unwrap();
        assert_eq!((report.sentences, report.dropped), (1, 1));
    }
}

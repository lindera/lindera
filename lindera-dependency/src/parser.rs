//! Parsing: finding, for each bunsetsu of a sentence, the bunsetsu it
//! depends on.
//!
//! The parser goes through the bunsetsu from left to right with a stack
//! (shift-reduce), and asks one question at a time: does this bunsetsu, the
//! source, depend on that one, the target? It is what CaboCha and J.DepP
//! do. Every head is to the right of its bunsetsu, no two arcs cross, and
//! the last bunsetsu is the root; the time is linear in the number of
//! bunsetsu.
//!
//! It is in three parts, so that each can be replaced:
//!
//! - **The features of a decision** are strings, with the names CaboCha
//!   gives them. They are made from the head and the function word of each
//!   bunsetsu, through the [`Schema`] of the dictionary. A [`FeatureSet`]
//!   says which [`Group`]s of features are made.
//! - **The classifier** ([`Classifier`]) scores the features of a decision:
//!   the source depends on the target when the score is above 0.
//!   [`LinearClassifier`] adds the weights of the features and of their
//!   pairs.
//! - **The loop** asks the decisions. With a classifier it parses
//!   ([`Parser::parse`]); with the answer of a sentence it gives the
//!   training instances of a classifier ([`instances`]).
//!
//! Parsing needs nothing but this crate.
//!
//! # Examples
//!
//! A classifier made by hand: a bunsetsu that ends in を depends on the
//! next one, and nothing else is attached before the end of the sentence.
//!
//! ```
//! use std::collections::HashMap;
//!
//! use lindera_dependency::lattice::LatticeMorpheme;
//! use lindera_dependency::parser::{FeatureSet, LinearClassifier, Parser};
//! use lindera_dependency::Schema;
//!
//! // SFS is the surface of the function word of the source, and DIST:1
//! // says that the target is the next bunsetsu.
//! let pairs = HashMap::from([(("SFS:を".to_string(), "DIST:1".to_string()), 2.0)]);
//! let classifier = LinearClassifier::new(-1.0, HashMap::new(), pairs)?;
//! let parser = Parser::new(Schema::ipadic(), FeatureSet::cabocha(), classifier);
//!
//! // 彼 は / 本 を / 読ん だ / と / 言う
//! let morphemes = [
//!     LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
//!     LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
//!     LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
//!     LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
//!     LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
//!     LatticeMorpheme::new("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
//!     LatticeMorpheme::new("と", "助詞,格助詞,引用,*,*,*,と,ト,ト"),
//!     LatticeMorpheme::new("言う", "動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ"),
//! ];
//! let bunsetsu = parser.parse(&morphemes, &[0..2, 2..4, 4..7, 7..8])?;
//!
//! let heads: Vec<Option<usize>> = bunsetsu.iter().map(|one| one.head).collect();
//! assert_eq!(heads, [Some(3), Some(2), Some(3), None]);
//! // 本 を: the head word is 本, the function word を, and the score is
//! // the bias and the weight of the pair.
//! assert_eq!(bunsetsu[1].positions, Some((0, 1)));
//! assert_eq!(bunsetsu[1].score, Some(1.0));
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

mod classifier;
mod features;
mod oracle;
mod shift_reduce;

use std::ops::Range;

pub use classifier::{Classifier, LinearClassifier};
pub use features::{FeatureSet, Group};
pub use oracle::{Instance, instances, write_instances};

use crate::error::Result;
use crate::morpheme::Morpheme;
use crate::schema::Schema;
use crate::sentence::{Bunsetsu, check_bunsetsu};
use features::Prepared;
use shift_reduce::Decision;

/// Finds the head of each bunsetsu of a sentence.
///
/// A parser is made once and used from any number of threads: parsing takes
/// `&self` and keeps no state.
#[derive(Debug, Clone, PartialEq)]
pub struct Parser<C> {
    /// The schema of the dictionary of the morphemes.
    schema: Schema,
    /// The groups of features the classifier was trained on.
    features: FeatureSet,
    /// The classifier.
    classifier: C,
}

impl<C: Classifier> Parser<C> {
    /// Makes a parser.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morphemes.
    /// * `features` - The groups of features the classifier was trained on.
    /// * `classifier` - The classifier.
    ///
    /// # Returns
    ///
    /// The parser.
    pub fn new(schema: Schema, features: FeatureSet, classifier: C) -> Self {
        Self {
            schema,
            features,
            classifier,
        }
    }

    /// The schema of the parser.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The feature set of the parser.
    ///
    /// # Returns
    ///
    /// The feature set.
    pub fn feature_set(&self) -> &FeatureSet {
        &self.features
    }

    /// The classifier of the parser.
    ///
    /// # Returns
    ///
    /// The classifier.
    pub fn classifier(&self) -> &C {
        &self.classifier
    }

    /// Parses a sentence.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the sentence.
    /// * `bunsetsu` - Its bunsetsu from left to right, as ranges of indices
    ///   into `morphemes`: what
    ///   [`Chunker::chunk`](crate::chunker::Chunker::chunk) gives.
    ///
    /// # Returns
    ///
    /// The bunsetsu with their heads; the last one is the root. Each has
    /// the positions of its head word and of its function word, from the
    /// schema, and the score of the decision that attached it: 0 for the
    /// root and for a bunsetsu attached to the root, which is not asked.
    /// The result can be given to [`Sentence::new`](crate::Sentence::new).
    ///
    /// [`Error::Structure`](crate::Error::Structure) when the bunsetsu do
    /// not cover the morphemes from the first to the last without a gap, or
    /// one of them is empty. [`Error::Dictionary`](crate::Error::Dictionary)
    /// for a morpheme with fewer feature columns than the schema reads
    /// ([`Schema::check`]).
    pub fn parse<M: Morpheme>(
        &self,
        morphemes: &[M],
        bunsetsu: &[Range<usize>],
    ) -> Result<Vec<Bunsetsu>> {
        let mut parsed: Vec<Bunsetsu> = bunsetsu
            .iter()
            .map(|range| Bunsetsu::new(range.clone(), None))
            .collect();
        check_bunsetsu(morphemes.len(), &parsed)?;
        self.schema.check(morphemes)?;

        let prepared: Prepared<u32> = Prepared::new(
            &self.schema,
            &self.features,
            morphemes,
            bunsetsu,
            |feature| self.classifier.id(feature),
        );
        let mut features = Vec::new();
        let arcs = shift_reduce::parse(bunsetsu.len(), |source, target, children| {
            prepared.decision(source, target, children, &mut features);
            let score = self.classifier.score(&features);
            Decision {
                attach: score > 0.0,
                score,
            }
        });

        for (index, one) in parsed.iter_mut().enumerate() {
            one.head = arcs.heads[index];
            one.positions = self.schema.positions(&morphemes[one.morphemes.clone()]);
            one.score = one.positions.map(|_| arcs.scores[index]);
        }
        Ok(parsed)
    }
}

/// Sentences for the tests of the parser.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::ops::Range;

    use crate::lattice::LatticeMorpheme;

    /// 彼 は / 「 本 / 」 を / 静か に 、 / 読ん だ 。
    ///
    /// The brackets are cut apart on purpose: one bunsetsu opens, the next
    /// closes.
    pub(crate) fn sentence() -> (Vec<LatticeMorpheme>, Vec<Range<usize>>) {
        let morphemes = [
            ("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
            ("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
            ("「", "記号,括弧開,*,*,*,*,「,「,「"),
            ("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
            ("」", "記号,括弧閉,*,*,*,*,」,」,」"),
            ("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
            ("静か", "名詞,形容動詞語幹,*,*,*,*,静か,シズカ,シズカ"),
            ("に", "助詞,副詞化,*,*,*,*,に,ニ,ニ"),
            ("、", "記号,読点,*,*,*,*,、,、,、"),
            ("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
            ("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
            ("。", "記号,句点,*,*,*,*,。,。,。"),
        ]
        .iter()
        .map(|(surface, features)| LatticeMorpheme::new(*surface, *features))
        .collect();
        (morphemes, vec![0..2, 2..4, 4..6, 6..9, 9..12])
    }

    /// A sentence of `count` bunsetsu, each 本 / を.
    pub(crate) fn repeated(count: usize) -> (Vec<LatticeMorpheme>, Vec<Range<usize>>) {
        let mut morphemes = Vec::new();
        let mut ranges = Vec::new();
        for index in 0..count {
            morphemes.push(LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"));
            morphemes.push(LatticeMorpheme::new(
                "を",
                "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
            ));
            ranges.push(2 * index..2 * index + 2);
        }
        (morphemes, ranges)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::HashMap;

    use super::*;
    use crate::error::Error;
    use crate::lattice::LatticeMorpheme;
    use crate::sentence::Sentence;

    /// A classifier that knows every feature and counts its calls.
    struct Counting {
        /// The score it gives.
        score: f64,
        /// The number of decisions it scored.
        scores: Cell<usize>,
    }

    impl Counting {
        /// A classifier that gives one score to every decision.
        fn new(score: f64) -> Self {
            Self {
                score,
                scores: Cell::new(0),
            }
        }
    }

    impl Classifier for Counting {
        fn id(&self, _: &str) -> Option<u32> {
            Some(0)
        }

        fn score(&self, _: &[u32]) -> f64 {
            self.scores.set(self.scores.get() + 1);
            self.score
        }
    }

    /// A parser for IPADIC with every group of features.
    fn parser<C: Classifier>(classifier: C) -> Parser<C> {
        Parser::new(Schema::ipadic(), FeatureSet::cabocha(), classifier)
    }

    /// The heads of bunsetsu.
    fn heads(bunsetsu: &[Bunsetsu]) -> Vec<Option<usize>> {
        bunsetsu.iter().map(|one| one.head).collect()
    }

    #[test]
    fn a_pair_of_features_decides() {
        // A source that ends in を depends on the next bunsetsu, and on no
        // other: neither feature is enough alone.
        let pairs = HashMap::from([(("SFS:を".to_string(), "DIST:1".to_string()), 2.0)]);
        let classifier = LinearClassifier::new(-1.0, HashMap::new(), pairs).unwrap();
        let (morphemes, ranges) = fixtures::sentence();
        let parsed = parser(classifier).parse(&morphemes, &ranges).unwrap();
        // 彼は and 「本 are refused by the next bunsetsu; 」を is attached to
        // 静かに、; then 「本 is two bunsetsu from it, and refused.
        assert_eq!(heads(&parsed), [Some(4), Some(4), Some(3), Some(4), None]);
        let scores: Vec<Option<f64>> = parsed.iter().map(|one| one.score).collect();
        assert_eq!(
            scores,
            [Some(0.0), Some(0.0), Some(1.0), Some(0.0), Some(0.0)]
        );
    }

    #[test]
    fn the_result_is_a_sentence_without_violations() {
        let (morphemes, ranges) = fixtures::sentence();
        for score in [-1.0, 1.0] {
            let parsed = parser(Counting::new(score))
                .parse(&morphemes, &ranges)
                .unwrap();
            let sentence = Sentence::new(morphemes.clone(), parsed).unwrap();
            assert_eq!(sentence.violations(), [], "score {score}");
        }
    }

    #[test]
    fn a_score_above_zero_attaches() {
        let (morphemes, ranges) = fixtures::sentence();
        let attached = parser(Counting::new(0.25))
            .parse(&morphemes, &ranges)
            .unwrap();
        assert_eq!(heads(&attached), [Some(1), Some(2), Some(3), Some(4), None]);
        // A score of 0 does not.
        let refused = parser(Counting::new(0.0))
            .parse(&morphemes, &ranges)
            .unwrap();
        assert_eq!(heads(&refused), [Some(4), Some(4), Some(4), Some(4), None]);
    }

    #[test]
    fn the_positions_are_those_of_the_schema() {
        let (morphemes, ranges) = fixtures::sentence();
        let parsed = parser(Counting::new(1.0))
            .parse(&morphemes, &ranges)
            .unwrap();
        let positions: Vec<Option<(usize, usize)>> =
            parsed.iter().map(|one| one.positions).collect();
        assert_eq!(
            positions,
            [
                Some((0, 1)),
                Some((1, 1)),
                Some((0, 1)),
                Some((0, 1)),
                Some((0, 1))
            ]
        );
    }

    #[test]
    fn the_classifier_is_not_called_for_two_bunsetsu_or_fewer() {
        let (morphemes, _) = fixtures::sentence();
        let classifier = Counting::new(-1.0);
        let none = parser(&classifier)
            .parse::<LatticeMorpheme>(&[], &[])
            .unwrap();
        assert_eq!(none, []);

        #[allow(clippy::single_range_in_vec_init)]
        let one = parser(&classifier).parse(&morphemes[..2], &[0..2]).unwrap();
        assert_eq!(heads(&one), [None]);
        assert_eq!(one[0].score, Some(0.0));

        let two = parser(&classifier)
            .parse(&morphemes[..4], &[0..2, 2..4])
            .unwrap();
        assert_eq!(heads(&two), [Some(1), None]);
        assert_eq!(classifier.scores.get(), 0);
    }

    #[test]
    fn bunsetsu_that_do_not_cover_the_morphemes_are_refused() {
        let (morphemes, _) = fixtures::sentence();
        let classifier = Counting::new(1.0);
        // A gap, an end past the morphemes, an empty bunsetsu, and
        // morphemes left over.
        #[allow(clippy::single_range_in_vec_init)]
        let wrong: [Vec<Range<usize>>; 4] = [
            vec![0..2, 3..12],
            vec![0..2, 2..13],
            vec![0..2, 2..2, 2..12],
            vec![0..2],
        ];
        let accepted: Vec<usize> = wrong
            .iter()
            .enumerate()
            .filter(|(_, ranges)| {
                !matches!(
                    parser(&classifier).parse(&morphemes, ranges),
                    Err(Error::Structure(_))
                )
            })
            .map(|(index, _)| index)
            .collect();
        assert!(accepted.is_empty(), "not refused: {accepted:?}");
    }

    #[test]
    fn a_feature_the_classifier_does_not_know_is_left_out() {
        // The classifier knows the distance only: with a weight on DIST:1,
        // every bunsetsu depends on the next.
        let features = HashMap::from([("DIST:1".to_string(), 1.0)]);
        let classifier = LinearClassifier::new(-0.5, features, HashMap::new()).unwrap();
        let (morphemes, ranges) = fixtures::sentence();
        let parsed = parser(classifier).parse(&morphemes, &ranges).unwrap();
        assert_eq!(heads(&parsed), [Some(1), Some(2), Some(3), Some(4), None]);
    }

    #[test]
    fn a_group_left_out_of_the_set_is_not_made() {
        // Without the distance, the weight on DIST:1 is never reached.
        let features = HashMap::from([("DIST:1".to_string(), 1.0)]);
        let classifier = LinearClassifier::new(-0.5, features, HashMap::new()).unwrap();
        let set = FeatureSet::parse("source,target").unwrap();
        let (morphemes, ranges) = fixtures::sentence();
        let parsed = Parser::new(Schema::ipadic(), set, classifier)
            .parse(&morphemes, &ranges)
            .unwrap();
        assert_eq!(heads(&parsed), [Some(4), Some(4), Some(4), Some(4), None]);
    }

    #[test]
    fn a_morpheme_of_another_dictionary_is_refused() {
        let (mut morphemes, ranges) = fixtures::sentence();
        morphemes[1] = LatticeMorpheme::new("は", "助詞,係助詞");
        let classifier = LinearClassifier::new(1.0, HashMap::new(), HashMap::new()).unwrap();
        let error = parser(classifier).parse(&morphemes, &ranges).unwrap_err();
        assert!(matches!(error, crate::Error::Dictionary(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("the morpheme 2 (\"は\")"),
            "{error}"
        );
    }
}

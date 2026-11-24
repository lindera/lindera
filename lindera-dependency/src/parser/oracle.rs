//! Training instances: the decisions of the parser on a sentence whose
//! heads are known.
//!
//! The loop of the parser runs with the answer deciding in the place of a
//! classifier, as CaboCha does in its training mode. Each decision it asks
//! is one instance: the features the parser would have at that point, and
//! whether the source depends on the target. The features of the children
//! are those of the answer so far.

use std::io::Write;
use std::ops::Range;
use std::rc::Rc;

use super::features::{FeatureSet, Prepared};
use super::shift_reduce::{self, Decision};
use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::schema::Schema;
use crate::sentence::Sentence;

/// One decision of the parser, with its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// Whether the source depends on the target.
    pub label: bool,
    /// The feature strings of the decision, sorted, each once.
    pub features: Vec<String>,
}

/// The training instances of a sentence.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morphemes.
/// * `set` - The groups of features to make.
/// * `sentence` - The sentence, with the head of every bunsetsu.
///
/// # Returns
///
/// The instances, in the order the parser asks the decisions; none for a
/// sentence of two bunsetsu or fewer, where nothing is asked.
/// [`Error::Structure`] when the sentence does not pass
/// [`Sentence::check`], or when its tree is not one the parser can make
/// ([`Sentence::violations`]); [`Error::Dictionary`] for a morpheme with
/// fewer feature columns than the schema reads ([`Schema::check`]).
pub fn instances<M: Morpheme>(
    schema: &Schema,
    set: &FeatureSet,
    sentence: &Sentence<M>,
) -> Result<Vec<Instance>> {
    sentence.check()?;
    schema.check(&sentence.morphemes)?;
    if let Some(violation) = sentence.violations().first() {
        return Err(Error::Structure(format!(
            "the parser cannot make the tree of the sentence: {violation}"
        )));
    }
    let ranges: Vec<Range<usize>> = sentence
        .bunsetsu
        .iter()
        .map(|bunsetsu| bunsetsu.morphemes.clone())
        .collect();
    let answer: Vec<Option<usize>> = sentence
        .bunsetsu
        .iter()
        .map(|bunsetsu| bunsetsu.head)
        .collect();

    let prepared: Prepared<Rc<str>> =
        Prepared::new(schema, set, &sentence.morphemes, &ranges, |feature| {
            Some(Rc::from(feature))
        });
    let mut instances = Vec::new();
    let mut features = Vec::new();
    let arcs = shift_reduce::parse(ranges.len(), |source, target, children| {
        prepared.decision(source, target, children, &mut features);
        let label = answer[source] == Some(target);
        instances.push(Instance {
            label,
            features: features.iter().map(|feature| feature.to_string()).collect(),
        });
        Decision {
            attach: label,
            score: 0.0,
        }
    });
    // A tree without a violation is given back as it is: anything else is a
    // fault of the loop, and instances from it would teach the wrong
    // decisions.
    if arcs.heads != answer {
        return Err(Error::Structure(
            "the answer of the sentence did not give its tree back".to_string(),
        ));
    }
    Ok(instances)
}

/// Writes the training instances of a sentence.
///
/// One line for each instance: `+1` when the source depends on the target
/// and `-1` when it does not, then its feature strings, all separated by
/// tabs. An empty line ends the sentence, so that a file can be cut into
/// folds between sentences.
///
/// # Arguments
///
/// * `writer` - Where to write.
/// * `instances` - The instances of one sentence.
///
/// # Returns
///
/// `Ok(())`, [`Error::Structure`] for a feature that holds a tab or a line
/// break, or [`Error::Io`].
pub fn write_instances<W: Write>(writer: &mut W, instances: &[Instance]) -> Result<()> {
    for instance in instances {
        write!(writer, "{}", if instance.label { "+1" } else { "-1" })?;
        for feature in &instance.features {
            if feature.contains(['\t', '\n', '\r']) {
                return Err(Error::Structure(format!(
                    "the feature {feature:?} holds a tab or a line break"
                )));
            }
            write!(writer, "\t{feature}")?;
        }
        writeln!(writer)?;
    }
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::LatticeMorpheme;
    use crate::parser::fixtures;
    use crate::sentence::Bunsetsu;

    /// The sentence of the fixtures with heads.
    fn sentence(heads: &[Option<usize>]) -> Sentence<LatticeMorpheme> {
        let (morphemes, ranges) = fixtures::sentence();
        Sentence {
            comments: Vec::new(),
            morphemes,
            bunsetsu: ranges
                .into_iter()
                .zip(heads)
                .map(|(range, head)| Bunsetsu::new(range, *head))
                .collect(),
        }
    }

    /// The instances of a sentence, with every group of features.
    fn all(sentence: &Sentence<LatticeMorpheme>) -> Result<Vec<Instance>> {
        instances(&Schema::ipadic(), &FeatureSet::cabocha(), sentence)
    }

    /// The features of an instance that start with a prefix.
    fn starting<'a>(instance: &'a Instance, prefix: &str) -> Vec<&'a str> {
        instance
            .features
            .iter()
            .map(String::as_str)
            .filter(|feature| feature.starts_with(prefix))
            .collect()
    }

    #[test]
    fn the_instances_are_the_decisions_of_the_answer() {
        // 彼は -> 「本, 「本 -> 静かに、, 」を -> 静かに、, 静かに、 -> 読んだ。
        let answer = sentence(&[Some(1), Some(3), Some(3), Some(4), None]);
        let instances = all(&answer).unwrap();
        // (0, 1), (1, 2), (2, 3), (1, 3): the source is named by the
        // surface of its head, the target by the one of its first morpheme.
        let decisions: Vec<(Vec<&str>, Vec<&str>, bool)> = instances
            .iter()
            .map(|one| (starting(one, "SHS:"), starting(one, "DLS:"), one.label))
            .collect();
        assert_eq!(
            decisions,
            [
                (vec!["SHS:彼"], vec!["DLS:「"], true),
                (vec!["SHS:本"], vec!["DLS:」"], false),
                (vec!["SHS:」"], vec!["DLS:静か"], true),
                (vec!["SHS:本"], vec!["DLS:静か"], true),
            ]
        );
    }

    #[test]
    fn the_children_of_an_instance_are_those_of_the_answer_so_far() {
        let answer = sentence(&[Some(1), Some(3), Some(3), Some(4), None]);
        let instances = all(&answer).unwrap();
        let children: Vec<Vec<&str>> = instances
            .iter()
            .map(|one| {
                let mut children = starting(one, "a:");
                children.extend(starting(one, "A:"));
                children
            })
            .collect();
        assert_eq!(
            children,
            [
                vec![],
                // 彼は is attached to the source 「本.
                vec!["a:は"],
                vec![],
                // And 」を to the target 静かに、 by now.
                vec!["a:は", "A:を"],
            ]
        );
    }

    #[test]
    fn the_features_are_those_of_the_feature_set() {
        let answer = sentence(&[Some(1), Some(3), Some(3), Some(4), None]);
        let set = FeatureSet::parse("distance").unwrap();
        let instances = instances(&Schema::ipadic(), &set, &answer).unwrap();
        let features: Vec<&[String]> = instances
            .iter()
            .map(|one| one.features.as_slice())
            .collect();
        assert_eq!(features, [["DIST:1"], ["DIST:1"], ["DIST:1"], ["DIST:2-5"]]);
    }

    #[test]
    fn nothing_is_asked_for_two_bunsetsu_or_fewer() {
        let (morphemes, _) = fixtures::sentence();
        let two = Sentence::new(
            morphemes[..4].to_vec(),
            vec![Bunsetsu::new(0..2, Some(1)), Bunsetsu::new(2..4, None)],
        )
        .unwrap();
        assert_eq!(all(&two).unwrap(), []);
        let none = Sentence::new(morphemes[..4].to_vec(), Vec::new()).unwrap();
        assert_eq!(all(&none).unwrap(), []);
    }

    #[test]
    fn a_tree_the_parser_cannot_make_is_refused() {
        let refused = [
            // Crossing arcs: 0 -> 2 and 1 -> 3.
            (
                "crossing",
                sentence(&[Some(2), Some(3), Some(3), Some(4), None]),
            ),
            // A head to the left.
            (
                "left",
                sentence(&[Some(1), Some(0), Some(3), Some(4), None]),
            ),
            // The root is not the last bunsetsu.
            (
                "root",
                sentence(&[Some(1), Some(3), Some(3), None, Some(3)]),
            ),
            // A head that is no bunsetsu.
            (
                "range",
                sentence(&[Some(1), Some(7), Some(3), Some(4), None]),
            ),
        ];
        let accepted: Vec<&str> = refused
            .iter()
            .filter(|(_, sentence)| !matches!(all(sentence), Err(Error::Structure(_))))
            .map(|(name, _)| *name)
            .collect();
        assert!(accepted.is_empty(), "not refused: {accepted:?}");

        // The first three are refused for their tree, and the error says
        // so.
        let unnamed: Vec<&str> = refused[..3]
            .iter()
            .filter(|(_, sentence)| {
                !all(sentence)
                    .unwrap_err()
                    .to_string()
                    .contains("the parser cannot make the tree of the sentence")
            })
            .map(|(name, _)| *name)
            .collect();
        assert!(unnamed.is_empty(), "no reason given: {unnamed:?}");
    }

    #[test]
    fn a_sentence_whose_bunsetsu_do_not_fit_is_refused() {
        // The last bunsetsu ends past the morphemes.
        let mut broken = sentence(&[Some(1), Some(3), Some(3), Some(4), None]);
        broken.bunsetsu[4].morphemes = 9..13;
        assert!(matches!(all(&broken), Err(Error::Structure(_))));
    }

    #[test]
    fn instances_are_written_one_per_line() {
        let written = |instances: &[Instance]| -> Result<String> {
            let mut bytes = Vec::new();
            write_instances(&mut bytes, instances)?;
            Ok(String::from_utf8(bytes).unwrap())
        };
        let instance = |label: bool, features: &[&str]| Instance {
            label,
            features: features.iter().map(|feature| feature.to_string()).collect(),
        };
        assert_eq!(
            written(&[
                instance(true, &["DIST:1", "SFS:は"]),
                instance(false, &["DIST:2-5"]),
            ])
            .unwrap(),
            "+1\tDIST:1\tSFS:は\n-1\tDIST:2-5\n\n"
        );
        // A sentence without a decision is an empty line.
        assert_eq!(written(&[]).unwrap(), "\n");
        for feature in ["SHS:a\tb", "SHS:a\nb"] {
            assert!(
                matches!(
                    written(&[instance(true, &[feature])]),
                    Err(Error::Structure(_))
                ),
                "{feature:?}"
            );
        }
    }

    #[test]
    fn a_morpheme_of_another_dictionary_is_refused() {
        let mut sentence = sentence(&[Some(4), Some(2), Some(4), Some(4), None]);
        sentence.morphemes[0] = LatticeMorpheme::new("彼", "名詞,代名詞");
        let error = all(&sentence).unwrap_err();
        assert!(matches!(error, Error::Dictionary(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("the morpheme 1 (\"彼\")"),
            "{error}"
        );
    }
}

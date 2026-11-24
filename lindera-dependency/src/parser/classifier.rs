//! The classifier of the parser: it scores the features of a decision.
//!
//! A classifier works in two steps, as the model of CaboCha does. It first
//! gives a number to a feature string it knows ([`Classifier::id`]); the
//! parser does this once for each bunsetsu of a sentence. It then scores the
//! numbers of a decision ([`Classifier::score`]), without looking at a
//! string.

use std::collections::HashMap;

use crate::error::{Error, Result};

/// Scores the features of a decision: a source depends on a target when the
/// score is above 0.
pub trait Classifier {
    /// The number of a feature.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    ///
    /// # Returns
    ///
    /// The number, or `None` for a feature the classifier does not know,
    /// which the parser then leaves out.
    fn id(&self, feature: &str) -> Option<u32>;

    /// The score of a decision.
    ///
    /// # Arguments
    ///
    /// * `features` - The numbers of the features of the decision, in
    ///   ascending order, each once.
    ///
    /// # Returns
    ///
    /// The score: above 0 when the source depends on the target.
    fn score(&self, features: &[u32]) -> f64;
}

impl<C: Classifier + ?Sized> Classifier for &C {
    fn id(&self, feature: &str) -> Option<u32> {
        (**self).id(feature)
    }

    fn score(&self, features: &[u32]) -> f64 {
        (**self).score(features)
    }
}

/// A linear classifier over features and pairs of features, with its
/// weights in memory.
///
/// The score of a decision is the bias, plus the weight of each of its
/// features, plus the weight of each pair of them. This is what the
/// polynomial kernel of degree 2 of CaboCha and of J.DepP comes to once it
/// is expanded.
///
/// It looks up every pair of the features of a decision in a hash table,
/// which is enough to parse with weights made by hand or by an experiment.
/// A trained model has its own table.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearClassifier {
    /// The number of each feature.
    ids: HashMap<String, u32>,
    /// The bias.
    bias: f64,
    /// The weight of each feature, by its number.
    weights: Vec<f64>,
    /// The weight of each pair of features, the smaller number first.
    pairs: HashMap<(u32, u32), f64>,
}

impl LinearClassifier {
    /// Makes a classifier from its weights.
    ///
    /// # Arguments
    ///
    /// * `bias` - The bias.
    /// * `features` - The weight of each feature.
    /// * `pairs` - The weight of each pair of features, in either order. A
    ///   feature named here only has the weight 0 on its own.
    ///
    /// # Returns
    ///
    /// The classifier, or [`Error::Parser`] for a weight that is not
    /// finite, a pair of a feature with itself, or a pair given in both
    /// orders.
    pub fn new(
        bias: f64,
        features: HashMap<String, f64>,
        pairs: HashMap<(String, String), f64>,
    ) -> Result<Self> {
        if !bias.is_finite() {
            return Err(Error::Parser(format!("the bias {bias} is not finite")));
        }
        // The numbers follow the order of the names, so that the same
        // weights always give the same classifier.
        let mut names: Vec<&str> = features
            .keys()
            .map(String::as_str)
            .chain(
                pairs
                    .keys()
                    .flat_map(|(one, other)| [one.as_str(), other.as_str()]),
            )
            .collect();
        names.sort_unstable();
        names.dedup();
        let ids: HashMap<String, u32> = names
            .iter()
            .enumerate()
            .map(|(id, name)| (name.to_string(), id as u32))
            .collect();

        let mut weights = vec![0.0; names.len()];
        for (name, weight) in &features {
            if !weight.is_finite() {
                return Err(Error::Parser(format!(
                    "the weight {weight} of {name:?} is not finite"
                )));
            }
            weights[ids[name] as usize] = *weight;
        }

        let mut by_id = HashMap::new();
        for ((one, other), weight) in &pairs {
            if !weight.is_finite() {
                return Err(Error::Parser(format!(
                    "the weight {weight} of the pair {one:?} and {other:?} is not finite"
                )));
            }
            if one == other {
                return Err(Error::Parser(format!(
                    "{one:?} is paired with itself: give its weight as a feature"
                )));
            }
            let (low, high) = (ids[one].min(ids[other]), ids[one].max(ids[other]));
            if by_id.insert((low, high), *weight).is_some() {
                return Err(Error::Parser(format!(
                    "the pair {one:?} and {other:?} is given in both orders"
                )));
            }
        }

        Ok(Self {
            ids,
            bias,
            weights,
            pairs: by_id,
        })
    }

    /// The number of features the classifier knows.
    ///
    /// # Returns
    ///
    /// The number of features with a weight or in a pair.
    pub fn feature_count(&self) -> usize {
        self.weights.len()
    }

    /// The number of pairs of features with a weight.
    ///
    /// # Returns
    ///
    /// The number of pairs.
    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }
}

impl Classifier for LinearClassifier {
    fn id(&self, feature: &str) -> Option<u32> {
        self.ids.get(feature).copied()
    }

    fn score(&self, features: &[u32]) -> f64 {
        let mut score = self.bias;
        for (index, &one) in features.iter().enumerate() {
            score += self.weights.get(one as usize).copied().unwrap_or(0.0);
            if self.pairs.is_empty() {
                continue;
            }
            for &other in &features[index + 1..] {
                let pair = (one.min(other), one.max(other));
                score += self.pairs.get(&pair).copied().unwrap_or(0.0);
            }
        }
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A classifier with the bias 0.5, three features and one pair.
    fn classifier() -> LinearClassifier {
        let features = HashMap::from([
            ("a".to_string(), 1.0),
            ("b".to_string(), -2.0),
            ("c".to_string(), 4.0),
        ]);
        let pairs = HashMap::from([(("c".to_string(), "a".to_string()), 8.0)]);
        LinearClassifier::new(0.5, features, pairs).unwrap()
    }

    /// The numbers of features, sorted.
    fn ids(classifier: &LinearClassifier, names: &[&str]) -> Vec<u32> {
        let mut ids: Vec<u32> = names
            .iter()
            .map(|name| classifier.id(name).unwrap())
            .collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn the_score_adds_the_bias_the_features_and_the_pairs() {
        let classifier = classifier();
        assert_eq!(classifier.score(&[]), 0.5);
        assert_eq!(classifier.score(&ids(&classifier, &["a"])), 1.5);
        assert_eq!(classifier.score(&ids(&classifier, &["a", "b"])), -0.5);
        // The pair of a and c adds 8.
        assert_eq!(classifier.score(&ids(&classifier, &["a", "c"])), 13.5);
        assert_eq!(classifier.score(&ids(&classifier, &["a", "b", "c"])), 11.5);
    }

    #[test]
    fn an_unknown_feature_has_no_number() {
        let classifier = classifier();
        assert_eq!(classifier.id("d"), None);
        assert_eq!(classifier.feature_count(), 3);
        assert_eq!(classifier.pair_count(), 1);
    }

    #[test]
    fn a_feature_named_in_a_pair_only_has_a_number() {
        let pairs = HashMap::from([(("x".to_string(), "y".to_string()), 2.0)]);
        let classifier = LinearClassifier::new(0.0, HashMap::new(), pairs).unwrap();
        assert_eq!(classifier.feature_count(), 2);
        assert_eq!(classifier.score(&ids(&classifier, &["x"])), 0.0);
        assert_eq!(classifier.score(&ids(&classifier, &["x", "y"])), 2.0);
    }

    #[test]
    fn the_numbers_follow_the_names() {
        let classifier = classifier();
        assert_eq!(classifier.id("a"), Some(0));
        assert_eq!(classifier.id("b"), Some(1));
        assert_eq!(classifier.id("c"), Some(2));
    }

    #[test]
    fn weights_that_cannot_be_used_are_refused() {
        let feature = |weight: f64| HashMap::from([("a".to_string(), weight)]);
        let pair = |one: &str, other: &str, weight: f64| {
            HashMap::from([((one.to_string(), other.to_string()), weight)])
        };
        let mut both = pair("a", "b", 1.0);
        both.insert(("b".to_string(), "a".to_string()), 1.0);

        let refused = [
            (
                "bias",
                LinearClassifier::new(f64::NAN, HashMap::new(), HashMap::new()),
            ),
            (
                "feature",
                LinearClassifier::new(0.0, feature(f64::INFINITY), HashMap::new()),
            ),
            (
                "pair",
                LinearClassifier::new(0.0, HashMap::new(), pair("a", "b", f64::NAN)),
            ),
            (
                "itself",
                LinearClassifier::new(0.0, HashMap::new(), pair("a", "a", 1.0)),
            ),
            (
                "both orders",
                LinearClassifier::new(0.0, HashMap::new(), both),
            ),
        ];
        let accepted: Vec<&str> = refused
            .iter()
            .filter(|(_, result)| !matches!(result, Err(Error::Parser(_))))
            .map(|(name, _)| *name)
            .collect();
        assert!(accepted.is_empty(), "not refused: {accepted:?}");
    }
}

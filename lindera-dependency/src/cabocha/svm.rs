//! Reading the parser model of CaboCha, and expanding its support vectors
//! as CaboCha does.
//!
//! The file has three blocks, with an empty line between them:
//!
//! ```text
//! C: 0.001500             the header: `key: value` lines
//! degree: 2
//! posset: IPA
//!
//! 20800 A:あえて          the features: the number of each, and its string
//!
//! -0.0004940063455534 0 1 2 3 ...   the support vectors: the weight of
//!                                   each, and the numbers of its features
//! ```
//!
//! The score of a decision with the features `x` is `Σ α (1 + s)^2` over
//! the support vectors, where `s` is the number of features a support
//! vector shares with `x`. With `s = Σ x_f` and `x_f` 0 or 1,
//! `(1 + s)^2 = 1 + 3 Σ x_f + 2 Σ x_f x_g` over the features and the pairs
//! `f < g` of the support vector: a bias `Σ α`, a weight `3 α` for each of
//! its features and a weight `2 α` for each of its pairs.
//!
//! CaboCha does not parse with the text model. `cabocha-model-index`
//! compiles it (`FastSVMModel::compile` in `src/svm.cpp`), and [`expand`]
//! does what it does, step by step, so that a decision adds up the same
//! whole numbers:
//!
//! - the weights are added in 32-bit floats, in the order of the support
//!   vectors;
//! - a pair of two of the most frequent features (the numbers below
//!   `frequent_features`) is always kept. Another pair is kept when it is
//!   in `min_support` support vectors or more and its weight is not
//!   between `-sigma × negatives / all` and `sigma × positives / all`;
//! - every weight is divided by the unit, the largest absolute weight over
//!   1,048,575, and truncated to a whole number. So is the bias.

use std::collections::HashMap;

use super::Options;
use crate::error::{Error, Result};

/// The whole number the largest weight becomes: `kPKEBase` of CaboCha.
const BASE: f32 = 1_048_575.0;

/// A parser model of CaboCha, as the file has it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SvmModel {
    /// The header.
    pub(super) header: HashMap<String, String>,
    /// The string of each feature, by its number; empty for a number the
    /// file gives no string.
    pub(super) names: Vec<String>,
    /// The support vectors: the weight of each, and the numbers of its
    /// features, sorted.
    pub(super) vectors: Vec<(f64, Vec<u32>)>,
}

/// The support vectors of a model as weights of features and of pairs.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Expansion {
    /// What a decision starts from, before the weights are added: the
    /// truncated `Σ α`. CaboCha keeps its negative and subtracts it.
    pub(super) bias: i32,
    /// What the whole numbers are multiplied by to give a score: the
    /// normalize factor of CaboCha.
    pub(super) unit: f32,
    /// The weight of each feature, by its number.
    pub(super) weights: Vec<i32>,
    /// The pairs that are kept and whose weight is not 0: the two numbers,
    /// the smaller first, and the weight. Sorted.
    pub(super) pairs: Vec<((u32, u32), i32)>,
    /// The number of support vectors with a weight above 0.
    pub(super) positives: usize,
    /// The number of the other support vectors.
    pub(super) negatives: usize,
    /// The number of pairs of features in the support vectors.
    pub(super) all_pairs: usize,
    /// The number of those that are kept, with the ones whose weight is
    /// truncated to 0.
    pub(super) kept_pairs: usize,
}

/// Makes the error of a file that is not a parser model.
fn error(problem: impl std::fmt::Display) -> Error {
    Error::Cabocha(format!("the parser model {problem}"))
}

/// Reads a parser model of CaboCha.
///
/// # Arguments
///
/// * `text` - The content of the file, as `dep.ipa.txt`.
///
/// # Returns
///
/// The model, or [`Error::Cabocha`] for a file that is not a parser model
/// in text: a block is missing, a line is not what its block holds, a
/// support vector names a feature twice or one the file has no string for.
pub(super) fn read(text: &str) -> Result<SvmModel> {
    let mut lines = text.lines().map(|line| line.trim_end_matches('\r'));
    let mut header = HashMap::new();
    for line in lines.by_ref().take_while(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once(": ")
            .ok_or_else(|| error(format_args!("has the line {line:?} in its header")))?;
        header.insert(key.to_string(), value.to_string());
    }
    if header.is_empty() {
        return Err(error("has no header: it is not a model of CaboCha in text"));
    }

    let mut names: Vec<String> = Vec::new();
    for line in lines.by_ref().take_while(|line| !line.is_empty()) {
        let (number, name) = line
            .split_once([' ', '\t'])
            .filter(|(_, name)| !name.is_empty())
            .ok_or_else(|| error(format_args!("has the feature line {line:?}")))?;
        let number: usize = number
            .parse()
            .ok()
            .filter(|number| u32::try_from(*number).is_ok())
            .ok_or_else(|| error(format_args!("has the feature line {line:?}")))?;
        if names.len() <= number {
            names.resize(number + 1, String::new());
        }
        if !names[number].is_empty() {
            return Err(error(format_args!("has the feature {number} twice")));
        }
        names[number] = name.to_string();
    }
    if names.is_empty() {
        return Err(error("has no features"));
    }

    let mut vectors = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let mut fields = line.split(' ').filter(|field| !field.is_empty());
        let alpha: f64 = fields
            .next()
            .and_then(|field| field.parse().ok())
            .filter(|alpha: &f64| alpha.is_finite())
            .ok_or_else(|| error(format_args!("has the support vector {line:?}")))?;
        let mut features = Vec::new();
        for field in fields {
            let feature: u32 = field
                .parse()
                .map_err(|_| error(format_args!("has the support vector {line:?}")))?;
            if names.get(feature as usize).is_none_or(String::is_empty) {
                return Err(error(format_args!(
                    "has a support vector with the feature {feature}, which it has no string for"
                )));
            }
            features.push(feature);
        }
        features.sort_unstable();
        if features.is_empty() {
            return Err(error("has a support vector without features"));
        }
        if let Some(pair) = features.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(error(format_args!(
                "has a support vector with the feature {} twice",
                pair[0]
            )));
        }
        vectors.push((alpha, features));
    }
    if vectors.is_empty() {
        return Err(error("has no support vectors"));
    }
    Ok(SvmModel {
        header,
        names,
        vectors,
    })
}

/// Expands the support vectors of a model as `cabocha-model-index` does.
///
/// See the [module documentation](self) for the steps.
///
/// # Arguments
///
/// * `model` - The model.
/// * `options` - The pruning: `sigma`, `min_support` and
///   `frequent_features`.
///
/// # Returns
///
/// The weights, or [`Error::Cabocha`] for a model whose weights are all 0.
pub(super) fn expand(model: &SvmModel, options: &Options) -> Result<Expansion> {
    // One more than the largest feature of a support vector.
    let size = model
        .vectors
        .iter()
        .filter_map(|(_, features)| features.last())
        .max()
        .map_or(0, |largest| *largest as usize + 1);
    let frequent = options.frequent_features.min(size) as u64;

    // The bias, in a double as CaboCha has it, and the weights of the
    // features and of the pairs in 32-bit floats.
    let mut bias = 0.0f64;
    let mut weights = vec![0.0f32; size];
    // For a pair: in how many support vectors it is, counted in a byte that
    // wraps as CaboCha's does, and its weight.
    let mut pairs: HashMap<u64, (u8, f32)> = HashMap::new();
    let (mut positives, mut negatives) = (0usize, 0usize);
    for (alpha, features) in &model.vectors {
        if *alpha > 0.0 {
            positives += 1;
        } else {
            negatives += 1;
        }
        let alpha = *alpha as f32;
        bias += f64::from(alpha);
        for (index, &one) in features.iter().enumerate() {
            weights[one as usize] += 3.0 * alpha;
            for &other in &features[index + 1..] {
                let pair = pairs
                    .entry(u64::from(one) << 32 | u64::from(other))
                    .or_insert((0, 0.0));
                pair.0 = pair.0.wrapping_add(1);
                pair.1 += 2.0 * alpha;
            }
        }
    }

    let all = (positives + negatives) as f64;
    // In doubles, and then 32-bit floats, as the weights they are compared
    // with.
    let sigma_positive = (options.sigma * positives as f64 / all) as f32;
    let sigma_negative = (-options.sigma * negatives as f64 / all) as f32;
    let all_pairs = pairs.len();
    let mut kept: Vec<((u32, u32), f32)> = pairs
        .into_iter()
        .filter(|(key, (count, weight))| {
            let (one, other) = (key >> 32, key & 0xFFFF_FFFF);
            (one < frequent && other < frequent)
                || (usize::from(*count) >= options.min_support
                    && (*weight <= sigma_negative || *weight >= sigma_positive))
        })
        .map(|(key, (_, weight))| (((key >> 32) as u32, key as u32), weight))
        .collect();
    kept.sort_unstable_by_key(|(pair, _)| *pair);
    let kept_pairs = kept.len();

    let largest = weights
        .iter()
        .chain(kept.iter().map(|(_, weight)| weight))
        .fold(0.0f32, |largest, weight| largest.max(weight.abs()));
    let unit = largest / BASE;
    if !(unit > 0.0 && unit.is_finite()) {
        return Err(error("has no weight that is not 0"));
    }
    Ok(Expansion {
        bias: (bias / f64::from(unit)) as i32,
        unit,
        weights: weights
            .iter()
            .map(|weight| (weight / unit) as i32)
            .collect(),
        pairs: kept
            .into_iter()
            .map(|(pair, weight)| (pair, (weight / unit) as i32))
            .filter(|(_, weight)| *weight != 0)
            .collect(),
        positives,
        negatives,
        all_pairs,
        kept_pairs,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A small model: five features and four support vectors.
    pub(in crate::cabocha) const MODEL: &str = "\
C: 0.001500
bias: 0.000000
charset: UTF8
degree: 2
parsing_algorithm: 0
posset: IPA
type: dep

0 DIST:1
3 SHS:本
1 DHS:読む
2 SFS:を
4 GNB:1

0.5 0 1 2
-0.25 2 0 3
0.25 1 3
-0.125 0 4
";

    /// The pruning that keeps every pair: the kernel itself.
    pub(in crate::cabocha) fn unpruned() -> Options {
        Options {
            sigma: 0.0,
            min_support: 1,
            frequent_features: 0,
        }
    }

    /// The score of the kernel for a decision: `Σ α (1 + s)^2`.
    fn kernel(model: &SvmModel, features: &[u32]) -> f64 {
        model
            .vectors
            .iter()
            .map(|(alpha, vector)| {
                let shared = vector.iter().filter(|one| features.contains(one)).count() as f64;
                alpha * (1.0 + shared) * (1.0 + shared)
            })
            .sum()
    }

    /// The score of an expansion for a decision, as CaboCha adds it up.
    fn score(expansion: &Expansion, features: &[u32]) -> f64 {
        let mut sum = i64::from(expansion.bias);
        for (index, one) in features.iter().enumerate() {
            sum += i64::from(expansion.weights[*one as usize]);
            for other in &features[index + 1..] {
                if let Ok(found) = expansion
                    .pairs
                    .binary_search_by_key(&(*one, *other), |(pair, _)| *pair)
                {
                    sum += i64::from(expansion.pairs[found].1);
                }
            }
        }
        sum as f64 * f64::from(expansion.unit)
    }

    #[test]
    fn reads_the_blocks_of_a_model() {
        let model = read(MODEL).unwrap();
        assert_eq!(model.header["posset"], "IPA");
        assert_eq!(model.header["degree"], "2");
        assert_eq!(model.header.len(), 7);
        assert_eq!(
            model.names,
            ["DIST:1", "DHS:読む", "SFS:を", "SHS:本", "GNB:1"]
        );
        // The features of a support vector are sorted.
        assert_eq!(
            model.vectors,
            [
                (0.5, vec![0, 1, 2]),
                (-0.25, vec![0, 2, 3]),
                (0.25, vec![1, 3]),
                (-0.125, vec![0, 4]),
            ]
        );
        assert_eq!(read(&MODEL.replace('\n', "\r\n")).unwrap(), model);
    }

    #[test]
    fn the_weights_are_those_of_the_kernel_in_whole_numbers() {
        let model = read(MODEL).unwrap();
        let expansion = expand(&model, &unpruned()).unwrap();
        // 3 α for each feature of a support vector, 2 α for each pair, and
        // Σ α for the bias, before the unit: by hand.
        let features = [0.375, 2.25, 0.75, 0.0, -0.375];
        let pairs = [
            ((0, 1), 1.0),
            ((0, 2), 0.5),
            ((0, 3), -0.5),
            ((0, 4), -0.25),
            ((1, 2), 1.0),
            ((1, 3), 0.5),
            ((2, 3), -0.5),
        ];
        // The largest is 2.25: it becomes 1,048,575.
        let unit = 2.25f32 / BASE;
        assert_eq!(expansion.unit, unit);
        let whole = |weight: f32| (weight / unit) as i32;
        assert_eq!(expansion.weights, features.map(whole));
        assert_eq!(expansion.weights[1], 1_048_575);
        assert_eq!(
            expansion.pairs,
            pairs.map(|(pair, weight)| (pair, whole(weight)))
        );
        assert_eq!(expansion.bias, (0.375 / f64::from(unit)) as i32);
        assert_eq!((expansion.positives, expansion.negatives), (2, 2));
        assert_eq!((expansion.all_pairs, expansion.kept_pairs), (7, 7));

        // With every pair it is the kernel, up to the truncation: at most
        // one unit for the bias, each feature and each pair of a decision.
        for features in [
            vec![0, 1, 2, 3, 4],
            vec![0, 1, 2],
            vec![1, 3],
            vec![4],
            vec![2, 4],
            vec![],
        ] {
            let terms = 1 + features.len() + features.len() * features.len() / 2;
            let difference = (score(&expansion, &features) - kernel(&model, &features)).abs();
            assert!(
                difference <= terms as f64 * f64::from(unit),
                "{features:?}: {difference}"
            );
        }
    }

    #[test]
    fn a_weight_is_truncated_towards_zero() {
        // The largest weight is 3, and 0.51 is 178,257.75 units: 178,257
        // and -178,257, neither rounded nor taken down.
        let model = read("degree: 2\n\n0 a\n1 b\n2 c\n\n1 0\n0.17 1\n-0.17 2\n").unwrap();
        let expansion = expand(&model, &unpruned()).unwrap();
        assert_eq!(expansion.weights, [1_048_575, 178_257, -178_257]);

        // A pair is truncated as well: 2 is the largest weight here, and
        // -0.34 is -178,257.75 units.
        let model =
            read("degree: 2\n\n0 a\n1 b\n2 c\n\n1 0 1\n-0.17 1 2\n-0.5 0\n-0.5 1\n").unwrap();
        let expansion = expand(&model, &unpruned()).unwrap();
        assert_eq!(expansion.pairs, [((0, 1), 1_048_575), ((1, 2), -178_257)]);

        // A pair whose weight is less than a unit is not there, and is
        // counted among the ones that are kept.
        let model = read("degree: 2\n\n0 a\n1 b\n\n1 0\n0.0000001 0 1\n").unwrap();
        let expansion = expand(&model, &unpruned()).unwrap();
        assert!(expansion.pairs.is_empty(), "{:?}", expansion.pairs);
        assert_eq!(expansion.kept_pairs, 1);
    }

    #[test]
    fn a_pair_is_kept_by_its_frequency_and_its_weight() {
        // (0, 1) is in two support vectors and (0, 2), (1, 2) in one.
        let text = "degree: 2\n\n0 a\n1 b\n2 c\n\n0.5 0 1\n0.25 0 1 2\n-0.5 2\n";
        let model = read(text).unwrap();
        let pairs = |options: &Options| -> Vec<(u32, u32)> {
            expand(&model, options)
                .unwrap()
                .pairs
                .iter()
                .map(|(pair, _)| *pair)
                .collect()
        };
        assert_eq!(pairs(&unpruned()), [(0, 1), (0, 2), (1, 2)]);
        // In two support vectors or more.
        let twice = Options {
            min_support: 2,
            ..unpruned()
        };
        assert_eq!(pairs(&twice), [(0, 1)]);
        // A weight of 1.5 or more: two of three support vectors are
        // positive, so sigma 2.25 asks for 1.5, and 2.26 for more.
        let heavy = |sigma: f64| Options {
            sigma,
            ..unpruned()
        };
        assert_eq!(pairs(&heavy(2.25)), [(0, 1)]);
        assert_eq!(pairs(&heavy(2.26)), []);
        assert_eq!(pairs(&heavy(0.75)), [(0, 1), (0, 2), (1, 2)]);
        assert_eq!(pairs(&heavy(0.76)), [(0, 1)]);
        // A pair of two frequent features is kept whatever it is.
        let frequent = |frequent_features: usize| Options {
            sigma: 100.0,
            min_support: 100,
            frequent_features,
        };
        assert_eq!(pairs(&frequent(0)), []);
        assert_eq!(pairs(&frequent(1)), []);
        assert_eq!(pairs(&frequent(2)), [(0, 1)]);
        assert_eq!(pairs(&frequent(3)), [(0, 1), (0, 2), (1, 2)]);
        assert_eq!(pairs(&frequent(1000)), [(0, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn a_negative_pair_is_kept_by_the_share_of_the_negatives() {
        // One of four support vectors is negative: sigma 2 asks for -0.5
        // or less, and for 1.5 or more.
        let text = "degree: 2\n\n0 a\n1 b\n2 c\n3 d\n\n-0.25 0 1\n0.5 2 3\n0.25 2\n0.25 3\n";
        let model = read(text).unwrap();
        let pairs = |sigma: f64| -> Vec<(u32, u32)> {
            let options = Options {
                sigma,
                ..unpruned()
            };
            expand(&model, &options)
                .unwrap()
                .pairs
                .iter()
                .map(|(pair, _)| *pair)
                .collect()
        };
        assert_eq!(pairs(2.0), [(0, 1)]);
        assert_eq!(pairs(2.01), []);
        assert_eq!(pairs(4.0 / 3.0), [(0, 1), (2, 3)]);
    }

    #[test]
    fn the_unit_does_not_count_a_pair_that_is_dropped() {
        // The pair has the largest weight, 2, and the features 1.5.
        let model = read("degree: 2\n\n0 a\n1 b\n\n1 0 1\n-0.5 0\n-0.5 1\n").unwrap();
        let kept = expand(&model, &unpruned()).unwrap();
        assert_eq!(kept.unit, 2.0 / BASE);
        assert_eq!(kept.pairs, [((0, 1), 1_048_575)]);
        let dropped = expand(
            &model,
            &Options {
                min_support: 2,
                ..unpruned()
            },
        )
        .unwrap();
        assert_eq!(dropped.unit, 1.5 / BASE);
        assert_eq!(dropped.weights, [1_048_575, 1_048_575]);
        assert_eq!(dropped.bias, 0);
    }

    #[test]
    fn refuses_a_file_that_is_not_a_parser_model() {
        let message = |text: &str| read(text).unwrap_err().to_string();
        for (text, expected) in [
            (String::new(), "has no header"),
            ("0 B\n".to_string(), "has the line \"0 B\" in its header"),
            ("degree: 2\n".to_string(), "has no features"),
            ("degree: 2\n\n0 a\n".to_string(), "has no support vectors"),
            (
                MODEL.replace("3 SHS:本", "x SHS:本"),
                "has the feature line \"x SHS:本\"",
            ),
            (MODEL.replace("3 SHS:本", "3"), "has the feature line \"3\""),
            (
                MODEL.replace("3 SHS:本", "0 SHS:本"),
                "has the feature 0 twice",
            ),
            (
                MODEL.replace("0.25 1 3", "0.25 1 5"),
                "with the feature 5, which it has no string for",
            ),
            (
                MODEL.replace("3 SHS:本\n", ""),
                "with the feature 3, which it has no string for",
            ),
            (
                MODEL.replace("0.25 1 3", "0.25 1 1"),
                "with the feature 1 twice",
            ),
            (
                MODEL.replace("0.25 1 3", "x 1 3"),
                "has the support vector \"x 1 3\"",
            ),
            (
                MODEL.replace("0.25 1 3", "0.25 1 x"),
                "has the support vector \"0.25 1 x\"",
            ),
            (
                MODEL.replace("0.25 1 3", "inf 1 3"),
                "has the support vector \"inf 1 3\"",
            ),
            (
                MODEL.replace("0.25 1 3", "0.25"),
                "has a support vector without features",
            ),
        ] {
            let message = message(&text);
            assert!(
                message.starts_with("the parser model ") && message.contains(expected),
                "{expected}: {message}"
            );
        }
        let zero = read("degree: 2\n\n0 a\n\n0 0\n").unwrap();
        let message = expand(&zero, &unpruned()).unwrap_err().to_string();
        assert!(message.contains("has no weight that is not 0"), "{message}");
    }
}

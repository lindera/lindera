//! Training the classifier of the parser.
//!
//! The classifier is linear over a bias, the features of a decision and the
//! pairs of them, which is what a kernel `(1 + s)^2` comes to, `s` being the
//! number of features two decisions share: a step `d` on a decision adds `d`
//! to the bias, `3 d` to each of its features and `2 d` to each of their
//! pairs. Both reference implementations learn these weights, and the two
//! algorithms here are theirs:
//!
//! - [`Algorithm::DualCoordinateDescent`], as CaboCha does
//!   (`src/svm_learn.cpp`): a support vector machine with the hinge loss,
//!   solved in the dual, one decision at a time, with shrinking.
//! - [`Algorithm::PassiveAggressive`], as opal does for J.DepP (`pa.h`,
//!   PA-I): passes over the decisions in their order, each wrong or close
//!   one moving the weights by a step no larger than the cost.
//!
//! Where the weight of a pair is kept is separate from the algorithm
//! ([`Pairs`]): in an exact table, or in a table of a fixed size the pairs
//! are hashed into.
//!
//! The training instances are those of [`instances`]: the
//! decisions the parser asks when the answer of a sentence decides.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use super::features::FeatureSet;
use super::model::{PairWeights, ParserModel, bucket, feature_key};
use super::oracle::instances;
use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::schema::Schema;
use crate::sentence::Sentence;

/// The weight of a step on a feature, and on a pair of features: the
/// coefficients of `(1 + s)^2 = 1 + 3 s + 2 (s (s - 1) / 2)`.
const FEATURE: f64 = 3.0;
/// See [`FEATURE`].
const PAIR: f64 = 2.0;

/// The width of the projected gradient at which dual coordinate descent
/// stops, as in CaboCha.
const TOLERANCE: f64 = 0.1;

/// The largest number of passes of dual coordinate descent, as in CaboCha.
const MAX_PASSES: u32 = 5000;

/// How the weights are learned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Algorithm {
    /// Dual coordinate descent on a support vector machine with the hinge
    /// loss, as CaboCha trains.
    DualCoordinateDescent {
        /// The cost of a decision on the wrong side of its margin: the
        /// upper bound of what one decision adds to the weights. CaboCha's
        /// default is 0.0015.
        cost: f64,
    },
    /// Passive-aggressive (PA-I), as opal trains for J.DepP.
    PassiveAggressive {
        /// The upper bound of a step.
        cost: f64,
        /// The number of passes over the decisions.
        iterations: u32,
        /// Whether the weights are the average of those after every
        /// decision, and not the last ones.
        averaged: bool,
    },
}

/// Where the weight of a pair of features is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pairs {
    /// A weight of its own for every pair.
    Exact {
        /// The number of training decisions a pair has to be in to get a
        /// weight; 1 keeps every pair.
        min_support: u32,
    },
    /// A table of `2^bits` weights the pairs are hashed into; pairs that
    /// fall on the same weight share it.
    Hashed {
        /// The size of the table, as a power of two: 1 to 30.
        bits: u32,
    },
}

/// The settings of a training.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainOptions {
    /// The schema of the dictionary of the morphemes.
    pub schema: Schema,
    /// The groups of features.
    pub features: FeatureSet,
    /// The algorithm.
    pub algorithm: Algorithm,
    /// Where the weights of the pairs are kept.
    pub pairs: Pairs,
    /// The number of training decisions a feature has to be in to be used;
    /// 1 uses every feature.
    pub min_count: u32,
    /// With an exact table of the pairs: a pair whose weight is closer to
    /// 0 than this is left out of the model after the training; 0 leaves
    /// out only the pairs without a weight.
    pub pair_threshold: f64,
    /// The seed of the order in which dual coordinate descent visits the
    /// decisions. The same data and settings give the same model.
    pub seed: u64,
}

impl Default for TrainOptions {
    /// The schema of IPADIC, every group of features, and the algorithm and
    /// the table of the pairs that were chosen by cross-validation on UD
    /// Japanese-GSD.
    fn default() -> Self {
        Self {
            schema: Schema::ipadic(),
            features: FeatureSet::default(),
            algorithm: Algorithm::DualCoordinateDescent { cost: 0.0015 },
            pairs: Pairs::Hashed { bits: 22 },
            min_count: 1,
            pair_threshold: 0.0,
            seed: 1,
        }
    }
}

/// A trained model, and how the training went.
#[derive(Debug, Clone, PartialEq)]
pub struct Trained {
    /// The model.
    pub model: ParserModel,
    /// The number of sentences that gave a training decision.
    pub sentences: usize,
    /// The number of training decisions.
    pub instances: usize,
    /// The number of passes over the decisions.
    pub passes: u32,
    /// Whether the training ended by itself: `false` when dual coordinate
    /// descent used all its passes.
    pub converged: bool,
    /// The number of decisions that moved the weights: the support vectors
    /// of dual coordinate descent, the steps of passive-aggressive.
    pub updates: usize,
}

/// A training decision, by the places of its weights.
#[derive(Debug, Clone, PartialEq)]
struct Example {
    /// +1 when the source depends on the target, -1 when it does not.
    label: f64,
    /// The numbers of its features.
    features: Vec<u32>,
    /// The places of the weights of its pairs, sorted. A place is there
    /// once for each pair that falls on it.
    pairs: Vec<u32>,
}

/// The weights while they are learned. They hold the coefficients of the
/// kernel: the score of a decision is the sum of its weights.
#[derive(Debug, Clone, PartialEq)]
struct Weights {
    /// The bias.
    bias: f64,
    /// The weight of each feature.
    features: Vec<f32>,
    /// The weight at each place of the pairs.
    pairs: Vec<f32>,
}

impl Weights {
    /// Weights that are all 0.
    fn new(features: usize, pairs: usize) -> Self {
        Self {
            bias: 0.0,
            features: vec![0.0; features],
            pairs: vec![0.0; pairs],
        }
    }

    /// The score of a decision.
    fn score(&self, example: &Example) -> f64 {
        let mut score = self.bias;
        for &feature in &example.features {
            score += f64::from(self.features[feature as usize]);
        }
        for &pair in &example.pairs {
            score += f64::from(self.pairs[pair as usize]);
        }
        score
    }

    /// Moves the weights of a decision by a step.
    fn update(&mut self, example: &Example, step: f64) {
        self.bias += step;
        let feature = (FEATURE * step) as f32;
        for &index in &example.features {
            self.features[index as usize] += feature;
        }
        let pair = (PAIR * step) as f32;
        for &index in &example.pairs {
            self.pairs[index as usize] += pair;
        }
    }
}

/// A hasher for the keys of pairs, which are numbers already spread by
/// their making: one multiplication.
#[derive(Debug, Default, Clone, Copy)]
struct PairHasher(u64);

impl Hasher for PairHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (value ^ (value >> 29)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.0 ^= self.0 >> 32;
    }
}

/// A table from the key of a pair to a number.
type PairTable = HashMap<u64, u32, BuildHasherDefault<PairHasher>>;

/// The key of a pair of features in an exact table: the smaller number in
/// the upper half.
fn pair_key(one: u32, other: u32) -> u64 {
    (u64::from(one.min(other)) << 32) | u64::from(one.max(other))
}

/// Random numbers for the order of the decisions: SplitMix64, which needs
/// no dependency and gives the same numbers everywhere.
#[derive(Debug, Clone)]
struct Random(u64);

impl Random {
    /// The next number.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// Puts the first `count` numbers of a list in a random order.
    fn shuffle(&mut self, list: &mut [usize], count: usize) {
        for index in (1..count).rev() {
            let other = (self.next() % (index as u64 + 1)) as usize;
            list.swap(index, other);
        }
    }
}

/// The training decisions, with their features and pairs numbered.
#[derive(Debug, Clone, PartialEq)]
struct Data {
    /// The feature strings, sorted: the number of a feature is its index.
    names: Vec<String>,
    /// The decisions.
    examples: Vec<Example>,
    /// The pairs with a weight of their own, each with its place, when the
    /// table of the pairs is exact.
    exact: Option<Vec<(u64, u32)>>,
    /// The number of places of the weights of the pairs.
    places: usize,
    /// The number of sentences that gave a decision.
    sentences: usize,
}

/// Makes the training decisions of sentences.
fn prepare<M: Morpheme>(sentences: &[Sentence<M>], options: &TrainOptions) -> Result<Data> {
    if let Pairs::Hashed { bits } = options.pairs
        && !(1..=30).contains(&bits)
    {
        return Err(Error::Parser(format!(
            "the table of the pairs has 2^{bits} weights: 1 to 30 bits are allowed"
        )));
    }

    let mut decisions: Vec<(bool, Vec<String>)> = Vec::new();
    let mut with_decisions = 0;
    for (index, sentence) in sentences.iter().enumerate() {
        let made = instances(&options.schema, &options.features, sentence)
            .map_err(|error| Error::Parser(format!("sentence {}: {error}", index + 1)))?;
        with_decisions += usize::from(!made.is_empty());
        decisions.extend(made.into_iter().map(|one| (one.label, one.features)));
    }
    if decisions.is_empty() {
        return Err(Error::Parser(
            "the training data has no decision: every sentence has two bunsetsu or fewer"
                .to_string(),
        ));
    }

    // The features that are in enough decisions, numbered by their order.
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for (_, features) in &decisions {
        for feature in features {
            *counts.entry(feature.as_str()).or_insert(0) += 1;
        }
    }
    let mut names: Vec<&str> = counts
        .iter()
        .filter(|(_, count)| **count >= options.min_count)
        .map(|(name, _)| *name)
        .collect();
    names.sort_unstable();
    let numbers: HashMap<&str, u32> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, index as u32))
        .collect();
    let numbered: Vec<(f64, Vec<u32>)> = decisions
        .iter()
        .map(|(label, features)| {
            // Sorted strings give sorted numbers.
            let features = features
                .iter()
                .filter_map(|feature| numbers.get(feature.as_str()).copied())
                .collect();
            (if *label { 1.0 } else { -1.0 }, features)
        })
        .collect();

    let (examples, exact, places) = match options.pairs {
        Pairs::Exact { min_support } => {
            let mut support = PairTable::default();
            for (_, features) in &numbered {
                for (index, &one) in features.iter().enumerate() {
                    for &other in &features[index + 1..] {
                        *support.entry(pair_key(one, other)).or_insert(0) += 1;
                    }
                }
            }
            let mut kept: Vec<u64> = support
                .iter()
                .filter(|(_, count)| **count >= min_support)
                .map(|(key, _)| *key)
                .collect();
            drop(support);
            kept.sort_unstable();
            let mut place = PairTable::default();
            place.reserve(kept.len());
            for (index, &key) in kept.iter().enumerate() {
                place.insert(key, index as u32);
            }
            let examples = numbered
                .into_iter()
                .map(|(label, features)| {
                    let mut pairs = Vec::new();
                    for (index, &one) in features.iter().enumerate() {
                        for &other in &features[index + 1..] {
                            pairs.extend(place.get(&pair_key(one, other)).copied());
                        }
                    }
                    pairs.sort_unstable();
                    Example {
                        label,
                        features,
                        pairs,
                    }
                })
                .collect();
            let places = kept.len();
            let exact = kept
                .into_iter()
                .enumerate()
                .map(|(index, key)| (key, index as u32))
                .collect();
            (examples, Some(exact), places)
        }
        Pairs::Hashed { bits } => {
            let keys: Vec<u32> = names.iter().map(|name| feature_key(name)).collect();
            let examples = numbered
                .into_iter()
                .map(|(label, features)| {
                    let mut pairs = Vec::new();
                    for (index, &one) in features.iter().enumerate() {
                        for &other in &features[index + 1..] {
                            pairs.push(bucket(keys[one as usize], keys[other as usize], bits));
                        }
                    }
                    pairs.sort_unstable();
                    Example {
                        label,
                        features,
                        pairs,
                    }
                })
                .collect();
            (examples, None, 1usize << bits)
        }
    };

    Ok(Data {
        names: names.into_iter().map(str::to_string).collect(),
        examples,
        exact,
        places,
        sentences: with_decisions,
    })
}

/// What a training gave, before it is a model.
struct Learned {
    /// The weights.
    weights: Weights,
    /// The number of passes.
    passes: u32,
    /// Whether the training ended by itself.
    converged: bool,
    /// The number of decisions that moved the weights.
    updates: usize,
}

/// Dual coordinate descent, as `solveParameters` of CaboCha.
fn dual_coordinate_descent(data: &Data, cost: f64, seed: u64) -> Learned {
    let examples = &data.examples;
    let count = examples.len();
    let mut weights = Weights::new(data.names.len(), data.places);
    let mut alpha = vec![0.0f64; count];
    // The kernel of a decision with itself: 1, 3 for each feature, and 2
    // for each pair, times the square of how often its place is taken.
    let diagonal: Vec<f64> = examples
        .iter()
        .map(|example| {
            let mut pairs = 0.0;
            let mut index = 0;
            while index < example.pairs.len() {
                let mut end = index + 1;
                while end < example.pairs.len() && example.pairs[end] == example.pairs[index] {
                    end += 1;
                }
                let times = (end - index) as f64;
                pairs += times * times;
                index = end;
            }
            1.0 + FEATURE * example.features.len() as f64 + PAIR * pairs
        })
        .collect();

    let mut order: Vec<usize> = (0..count).collect();
    let mut active = count;
    let mut random = Random(seed);
    let mut max_old = f64::INFINITY;
    let mut min_old = f64::NEG_INFINITY;
    let mut passes = 0;
    let mut converged = false;
    while passes < MAX_PASSES {
        passes += 1;
        let mut max_new = f64::NEG_INFINITY;
        let mut min_new = f64::INFINITY;
        random.shuffle(&mut order, active);

        let mut position = 0;
        while position < active {
            let index = order[position];
            let example = &examples[index];
            let gradient = weights.score(example) * example.label - 1.0;
            let mut projected = 0.0;
            if alpha[index] == 0.0 {
                if gradient > max_old {
                    // The decision is on the right side for good: it is
                    // left out until every decision is looked at again.
                    active -= 1;
                    order.swap(position, active);
                    continue;
                } else if gradient < 0.0 {
                    projected = gradient;
                }
            } else if alpha[index] == cost {
                if gradient < min_old {
                    active -= 1;
                    order.swap(position, active);
                    continue;
                } else if gradient > 0.0 {
                    projected = gradient;
                }
            } else {
                projected = gradient;
            }
            max_new = max_new.max(projected);
            min_new = min_new.min(projected);

            if projected.abs() > 1.0e-12 {
                let old = alpha[index];
                alpha[index] = (old - gradient / diagonal[index]).clamp(0.0, cost);
                weights.update(example, (alpha[index] - old) * example.label);
            }
            position += 1;
        }

        if max_new - min_new <= TOLERANCE {
            if active == count {
                converged = true;
                break;
            }
            // Every decision is looked at once more.
            active = count;
            max_old = f64::INFINITY;
            min_old = f64::NEG_INFINITY;
            continue;
        }
        max_old = if max_new <= 0.0 {
            f64::INFINITY
        } else {
            max_new
        };
        min_old = if min_new >= 0.0 {
            f64::NEG_INFINITY
        } else {
            min_new
        };
    }

    Learned {
        weights,
        passes,
        converged,
        updates: alpha.iter().filter(|alpha| **alpha > 0.0).count(),
    }
}

/// Passive-aggressive (PA-I), as opal.
fn passive_aggressive(data: &Data, cost: f64, iterations: u32, averaged: bool) -> Learned {
    let mut weights = Weights::new(data.names.len(), data.places);
    // The sum of every step times the number of decisions seen before it:
    // what the average of the weights after every decision lacks.
    let mut late = Weights::new(
        if averaged { data.names.len() } else { 0 },
        if averaged { data.places } else { 0 },
    );
    let mut seen = 0.0f64;
    let mut updates = 0;
    for _ in 0..iterations {
        for example in &data.examples {
            let margin = weights.score(example) * example.label;
            if margin <= 1.0 {
                // The norm of opal is the number of features, also with
                // the pairs.
                let norm = example.features.len().max(1) as f64;
                let step = cost.min((1.0 - margin) / norm) * example.label;
                weights.update(example, step);
                if averaged {
                    late.update(example, step * seen);
                }
                updates += 1;
            }
            seen += 1.0;
        }
    }
    if averaged {
        weights.bias -= late.bias / seen;
        for (weight, late) in weights.features.iter_mut().zip(&late.features) {
            *weight -= (f64::from(*late) / seen) as f32;
        }
        for (weight, late) in weights.pairs.iter_mut().zip(&late.pairs) {
            *weight -= (f64::from(*late) / seen) as f32;
        }
    }
    Learned {
        weights,
        passes: iterations,
        converged: true,
        updates,
    }
}

/// Trains the classifier of the parser.
///
/// # Arguments
///
/// * `sentences` - The training data: sentences with the head of every
///   bunsetsu. A sentence of two bunsetsu or fewer gives no decision.
/// * `options` - The settings.
///
/// # Returns
///
/// The model and how the training went, or [`Error::Parser`]: a sentence
/// whose tree the parser cannot make or whose morphemes are not of the
/// dictionary of the schema (the message names the sentence), training
/// data without a decision, a cost that is not above 0, a threshold of the
/// pairs that is negative or not a number, no iteration, or a table of the
/// pairs of less than 1 or more than 30 bits.
pub fn train<M: Morpheme>(sentences: &[Sentence<M>], options: &TrainOptions) -> Result<Trained> {
    let cost = match options.algorithm {
        Algorithm::DualCoordinateDescent { cost } => cost,
        Algorithm::PassiveAggressive {
            cost, iterations, ..
        } => {
            if iterations == 0 {
                return Err(Error::Parser(
                    "passive-aggressive needs an iteration".to_string(),
                ));
            }
            cost
        }
    };
    if !(cost.is_finite() && cost > 0.0) {
        return Err(Error::Parser(format!(
            "the cost {cost} is not a number above 0"
        )));
    }

    if !(options.pair_threshold.is_finite() && options.pair_threshold >= 0.0) {
        return Err(Error::Parser(format!(
            "the threshold of the pairs {} is not a number of 0 or more",
            options.pair_threshold
        )));
    }

    let data = prepare(sentences, options)?;
    let learned = match options.algorithm {
        Algorithm::DualCoordinateDescent { cost } => {
            dual_coordinate_descent(&data, cost, options.seed)
        }
        Algorithm::PassiveAggressive {
            cost,
            iterations,
            averaged,
        } => passive_aggressive(&data, cost, iterations, averaged),
    };

    let pairs = match (&data.exact, options.pairs) {
        (Some(exact), _) => PairWeights::Exact(
            exact
                .iter()
                .map(|(key, place)| {
                    let weight = learned.weights.pairs[*place as usize];
                    let kept = f64::from(weight.abs()) >= options.pair_threshold;
                    (
                        ((key >> 32) as u32, *key as u32),
                        if kept { weight } else { 0.0 },
                    )
                })
                .collect(),
        ),
        (None, Pairs::Hashed { bits }) => PairWeights::Hashed {
            bits,
            weights: learned.weights.pairs.clone(),
        },
        (None, Pairs::Exact { .. }) => PairWeights::Exact(Vec::new()),
    };
    let features = data
        .names
        .iter()
        .cloned()
        .zip(learned.weights.features.iter().copied())
        .collect();
    let model = ParserModel::new(
        options.schema.clone(),
        options.features.clone(),
        learned.weights.bias,
        features,
        pairs,
    )?
    .compact();
    Ok(Trained {
        model,
        sentences: data.sentences,
        instances: data.examples.len(),
        passes: learned.passes,
        converged: learned.converged,
        updates: learned.updates,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::ops::Range;

    use super::*;
    use crate::lattice::LatticeMorpheme;
    use crate::model::{Model, ModelBuilder};
    use crate::parser::{Classifier, Parser, Quantization};
    use crate::sentence::Bunsetsu;

    /// The feature string of a word of the sentences of these tests.
    fn features_of(surface: &str) -> &'static str {
        match surface {
            "彼" => "名詞,代名詞,一般,*,*,*,彼,カレ,カレ",
            "犬" => "名詞,一般,*,*,*,*,犬,イヌ,イヌ",
            "本" => "名詞,一般,*,*,*,*,本,ホン,ホン",
            "水" => "名詞,一般,*,*,*,*,水,ミズ,ミズ",
            "花" => "名詞,一般,*,*,*,*,花,ハナ,ハナ",
            "は" => "助詞,係助詞,*,*,*,*,は,ハ,ワ",
            "が" => "助詞,格助詞,一般,*,*,*,が,ガ,ガ",
            "を" => "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ",
            "に" => "助詞,副詞化,*,*,*,*,に,ニ,ニ",
            "静か" => "名詞,形容動詞語幹,*,*,*,*,静か,シズカ,シズカ",
            "赤い" => "形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ",
            "読ん" => "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン",
            "飲ん" => "動詞,自立,*,*,五段・マ行,連用タ接続,飲む,ノン,ノン",
            "見" => "動詞,自立,*,*,一段,連用形,見る,ミ,ミ",
            "だ" => "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ",
            "た" => "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ",
            "。" => "記号,句点,*,*,*,*,。,。,。",
            other => panic!("no features for {other}"),
        }
    }

    /// A sentence: each bunsetsu as its words with spaces, and its head.
    fn sentence(bunsetsu: &[(&str, Option<usize>)]) -> Sentence<LatticeMorpheme> {
        let mut morphemes = Vec::new();
        let mut made = Vec::new();
        for (words, head) in bunsetsu {
            let start = morphemes.len();
            for word in words.split(' ') {
                morphemes.push(LatticeMorpheme::new(word, features_of(word)));
            }
            made.push(Bunsetsu::new(start..morphemes.len(), *head));
        }
        Sentence::new(morphemes, made).unwrap()
    }

    /// Six sentences; the first gives one decision, and the fifth and the
    /// sixth differ from the others in what an adjective and a verb before
    /// a noun depend on.
    pub(crate) fn training_data() -> Vec<Sentence<LatticeMorpheme>> {
        vec![
            sentence(&[("彼 は", Some(2)), ("本 を", Some(2)), ("読ん だ 。", None)]),
            sentence(&[
                ("彼 は", Some(3)),
                ("静か に", Some(3)),
                ("本 を", Some(3)),
                ("読ん だ 。", None),
            ]),
            sentence(&[("犬 が", Some(2)), ("水 を", Some(2)), ("飲ん だ 。", None)]),
            sentence(&[
                ("彼 は", Some(3)),
                ("赤い", Some(2)),
                ("花 を", Some(3)),
                ("見 た 。", None),
            ]),
            sentence(&[
                ("犬 が", Some(3)),
                ("赤い", Some(2)),
                ("本 を", Some(3)),
                ("読ん だ 。", None),
            ]),
            sentence(&[
                ("彼 は", Some(4)),
                ("犬 が", Some(2)),
                ("飲ん だ", Some(3)),
                ("水 を", Some(4)),
                ("見 た 。", None),
            ]),
        ]
    }

    /// Settings with a cost that lets a few decisions be learned.
    fn options(algorithm: Algorithm, pairs: Pairs) -> TrainOptions {
        TrainOptions {
            algorithm,
            pairs,
            ..TrainOptions::default()
        }
    }

    /// Dual coordinate descent with a cost of 1.
    const DCD: Algorithm = Algorithm::DualCoordinateDescent { cost: 1.0 };
    /// Passive-aggressive with 50 passes. Its step is the loss over the
    /// number of features, which with the pairs moves the score by about
    /// that number times too much: the cost has to bind.
    const PA: Algorithm = Algorithm::PassiveAggressive {
        cost: 0.001,
        iterations: 50,
        averaged: false,
    };
    /// A weight of its own for every pair.
    const EXACT: Pairs = Pairs::Exact { min_support: 1 };

    /// The ranges of the bunsetsu of a sentence.
    fn ranges(sentence: &Sentence<LatticeMorpheme>) -> Vec<Range<usize>> {
        sentence
            .bunsetsu
            .iter()
            .map(|bunsetsu| bunsetsu.morphemes.clone())
            .collect()
    }

    /// The number of sentences of the training data a model parses as the
    /// answer has them.
    fn reproduced(model: &ParserModel, options: &TrainOptions) -> usize {
        let parser = Parser::new(options.schema.clone(), options.features.clone(), model);
        training_data()
            .iter()
            .filter(|sentence| {
                let parsed = parser
                    .parse(&sentence.morphemes, &ranges(sentence))
                    .unwrap();
                parsed
                    .iter()
                    .zip(&sentence.bunsetsu)
                    .all(|(parsed, answer)| parsed.head == answer.head)
            })
            .count()
    }

    /// The message of the error of a training that is refused.
    fn refused(sentences: &[Sentence<LatticeMorpheme>], options: &TrainOptions) -> String {
        let error = train(sentences, options).unwrap_err();
        assert!(matches!(error, Error::Parser(_)), "{error:?}");
        error.to_string()
    }

    /// The numbers a model gives the features of the only decision of the
    /// first sentence, 彼 は -> 本 を, which the answer refuses.
    fn first_decision(model: &ParserModel, options: &TrainOptions) -> Vec<u32> {
        let made = instances(&options.schema, &options.features, &training_data()[0]).unwrap();
        assert_eq!(made.len(), 1);
        assert!(!made[0].label);
        let numbers: Vec<u32> = made[0]
            .features
            .iter()
            .filter_map(|feature| model.id(feature))
            .collect();
        assert_eq!(numbers.len(), made[0].features.len());
        numbers
    }

    #[test]
    fn both_algorithms_learn_the_training_data() {
        // With a sentence of two bunsetsu, which gives no decision.
        let mut training = training_data();
        training.push(sentence(&[("彼 は", Some(1)), ("読ん だ 。", None)]));
        let decisions: usize = training
            .iter()
            .map(|sentence| {
                instances(&Schema::ipadic(), &FeatureSet::default(), sentence)
                    .unwrap()
                    .len()
            })
            .sum();
        assert_eq!(decisions, 15);
        for algorithm in [DCD, PA] {
            for pairs in [EXACT, Pairs::Hashed { bits: 16 }] {
                let options = options(algorithm, pairs);
                let trained = train(&training, &options).unwrap();
                assert_eq!(
                    reproduced(&trained.model, &options),
                    6,
                    "{algorithm:?} {pairs:?}"
                );
                assert_eq!(training.len(), 7);
                assert_eq!(trained.sentences, 6);
                assert_eq!(trained.instances, decisions);
                assert!(trained.passes > 0);
                assert!(trained.converged);
                assert!(trained.updates > 0 && trained.updates <= decisions * 50);
                assert_eq!(trained.model.feature_set(), &options.features);
                assert_eq!(trained.model.schema(), &options.schema);
                assert!(trained.model.pair_count() > 0);
                // The same data and settings give the same model, and the
                // same bytes of a file.
                let again = train(&training, &options).unwrap();
                assert_eq!(again, trained);
                let bytes = |model: &ParserModel| {
                    ModelBuilder::new("test", options.schema.clone())
                        .unwrap()
                        .parser(model, Quantization::I8)
                        .unwrap()
                        .to_bytes()
                };
                assert_eq!(bytes(&again.model), bytes(&trained.model));
            }
        }
    }

    #[test]
    fn a_trained_model_parses_from_a_file() {
        let options = options(DCD, Pairs::Hashed { bits: 16 });
        let trained = train(&training_data(), &options).unwrap();
        for quantization in [Quantization::F32, Quantization::I16, Quantization::I8] {
            let bytes = ModelBuilder::new("test", options.schema.clone())
                .unwrap()
                .parser(&trained.model, quantization)
                .unwrap()
                .to_bytes();
            let model = Model::new(bytes).unwrap();
            let parser = model.parser().unwrap();
            assert_eq!(parser.feature_set(), &options.features);
            for sentence in training_data() {
                let parsed = parser
                    .parse(&sentence.morphemes, &ranges(&sentence))
                    .unwrap();
                let heads: Vec<Option<usize>> = parsed.iter().map(|one| one.head).collect();
                let answer: Vec<Option<usize>> =
                    sentence.bunsetsu.iter().map(|one| one.head).collect();
                assert_eq!(heads, answer, "{quantization:?}");
            }
        }
    }

    #[test]
    fn a_step_adds_the_coefficients_of_the_kernel() {
        let example = Example {
            label: 1.0,
            features: vec![0, 2],
            // Two pairs on the place 1, one on the place 3.
            pairs: vec![1, 1, 3],
        };
        let mut weights = Weights::new(3, 4);
        assert_eq!(weights.score(&example), 0.0);
        weights.update(&example, 0.5);
        assert_eq!(weights.bias, 0.5);
        assert_eq!(weights.features, [1.5, 0.0, 1.5]);
        assert_eq!(weights.pairs, [0.0, 2.0, 0.0, 1.0]);
        // 0.5 times 1, 3 for each of the two features, and 2 for each pair
        // times how often its place is taken: 2 squared, and 1.
        assert_eq!(weights.score(&example), 0.5 * (1.0 + 3.0 * 2.0 + 2.0 * 5.0));
        weights.update(&example, -0.5);
        assert_eq!(weights, Weights::new(3, 4));
    }

    #[test]
    fn dual_coordinate_descent_puts_one_decision_on_its_margin() {
        // One decision, whose answer is no. With a cost that does not
        // bind, one step puts it on its margin: the score is -1. This
        // holds only with the right kernel of the decision with itself,
        // also when its pairs share places.
        for pairs in [EXACT, Pairs::Hashed { bits: 16 }, Pairs::Hashed { bits: 3 }] {
            let options = options(Algorithm::DualCoordinateDescent { cost: 10.0 }, pairs);
            let trained = train(&training_data()[..1], &options).unwrap();
            assert_eq!(trained.instances, 1);
            assert_eq!(trained.updates, 1);
            assert!(trained.converged);
            let numbers = first_decision(&trained.model, &options);
            let score = trained.model.score(&numbers);
            assert!((score + 1.0).abs() < 1e-5, "{pairs:?}: {score}");
        }

        // With a cost that binds, the step is the cost: the bias is minus
        // the cost, and the score minus the cost times the kernel, which
        // without shared places is 1, 3 for each feature and 2 for each
        // pair.
        let options = options(Algorithm::DualCoordinateDescent { cost: 1e-4 }, EXACT);
        let trained = train(&training_data()[..1], &options).unwrap();
        assert_eq!(trained.model.bias(), -1e-4);
        let numbers = first_decision(&trained.model, &options);
        let count = numbers.len() as f64;
        let kernel = 1.0 + 3.0 * count + count * (count - 1.0);
        let score = trained.model.score(&numbers);
        assert!((score + 1e-4 * kernel).abs() < 1e-6, "{score} {kernel}");
        assert_eq!(kernel, (1.0 + count) * (1.0 + count));
    }

    #[test]
    fn passive_aggressive_steps_by_the_cost_or_by_the_loss_over_the_features() {
        let one = &training_data()[..1];
        let algorithm = |cost: f64, iterations: u32, averaged: bool| Algorithm::PassiveAggressive {
            cost,
            iterations,
            averaged,
        };
        // The cost binds: one step of the cost.
        let options_small = options(algorithm(1e-4, 1, false), EXACT);
        let trained = train(one, &options_small).unwrap();
        assert_eq!(trained.model.bias(), -1e-4);
        assert_eq!(trained.updates, 1);
        assert_eq!(trained.passes, 1);
        let count = first_decision(&trained.model, &options_small).len() as f64;
        // It does not bind: the loss, 1, over the number of features.
        let options_large = options(algorithm(10.0, 1, false), EXACT);
        let trained = train(one, &options_large).unwrap();
        assert_eq!(trained.model.bias(), -1.0 / count);
        // A second pass finds the decision past its margin, and does
        // nothing.
        let twice = train(one, &options(algorithm(10.0, 2, false), EXACT)).unwrap();
        assert_eq!(twice.model, trained.model);
        assert_eq!(twice.updates, 1);

        // The average of the weights after every decision is not the last
        // weights, and parses the training data as well.
        let training = training_data();
        let last = options(algorithm(0.001, 50, false), EXACT);
        let average = options(algorithm(0.001, 50, true), EXACT);
        let last_model = train(&training, &last).unwrap().model;
        let average_model = train(&training, &average).unwrap().model;
        assert_ne!(average_model, last_model);
        assert_eq!(reproduced(&average_model, &average), training.len());
        // One decision seen once: the average is the weights themselves.
        let averaged = train(one, &options(algorithm(10.0, 1, true), EXACT)).unwrap();
        assert_eq!(averaged.model, trained.model);
    }

    #[test]
    fn the_average_is_the_mean_of_the_weights_after_every_decision() {
        let options = options(PA, EXACT);
        let data = prepare(&training_data(), &options).unwrap();
        let (cost, iterations) = (0.001f64, 7u32);

        // The same passes, with the weights added up after every decision.
        let mut weights = Weights::new(data.names.len(), data.places);
        let mut bias = 0.0;
        let mut features = vec![0.0f64; data.names.len()];
        let mut pairs = vec![0.0f64; data.places];
        let mut seen = 0.0;
        for _ in 0..iterations {
            for example in &data.examples {
                let margin = weights.score(example) * example.label;
                if margin <= 1.0 {
                    let norm = example.features.len() as f64;
                    weights.update(example, cost.min((1.0 - margin) / norm) * example.label);
                }
                bias += weights.bias;
                for (sum, weight) in features.iter_mut().zip(&weights.features) {
                    *sum += f64::from(*weight);
                }
                for (sum, weight) in pairs.iter_mut().zip(&weights.pairs) {
                    *sum += f64::from(*weight);
                }
                seen += 1.0;
            }
        }
        assert_eq!(seen, (iterations as usize * data.examples.len()) as f64);

        let averaged = passive_aggressive(&data, cost, iterations, true).weights;
        let close = |one: f64, other: f64| (one - other).abs() < 1e-6;
        assert!(
            close(averaged.bias, bias / seen),
            "{} {}",
            averaged.bias,
            bias / seen
        );
        assert!(bias != 0.0 && !close(averaged.bias, weights.bias));
        for (averaged, sum) in averaged.features.iter().zip(&features) {
            assert!(
                close(f64::from(*averaged), sum / seen),
                "{averaged} {}",
                sum / seen
            );
        }
        for (averaged, sum) in averaged.pairs.iter().zip(&pairs) {
            assert!(
                close(f64::from(*averaged), sum / seen),
                "{averaged} {}",
                sum / seen
            );
        }
        assert!(features.iter().any(|sum| *sum != 0.0));
        assert!(pairs.iter().any(|sum| *sum != 0.0));
        // Without the average, the last weights.
        let last = passive_aggressive(&data, cost, iterations, false).weights;
        assert_eq!(last, weights);
    }

    #[test]
    fn features_and_pairs_below_their_counts_are_left_out() {
        let training = training_data();
        let all = train(&training, &options(DCD, EXACT)).unwrap().model;
        let frequent = train(
            &training,
            &TrainOptions {
                min_count: 3,
                ..options(DCD, EXACT)
            },
        )
        .unwrap()
        .model;
        assert!(frequent.feature_count() < all.feature_count());
        // 花 is in the two decisions of one sentence only.
        assert!(all.id("DHS:花").is_some());
        assert_eq!(frequent.id("DHS:花"), None);
        assert!(frequent.id("DIST:1").is_some());

        let supported = train(&training, &options(DCD, Pairs::Exact { min_support: 3 }))
            .unwrap()
            .model;
        assert!(supported.pair_count() < all.pair_count());
        assert!(supported.pair_count() > 0);

        // A threshold between the weights of the pairs: the middle one.
        let PairWeights::Exact(pairs) = all.pairs() else {
            panic!("the pairs are exact");
        };
        let mut sizes: Vec<f32> = pairs.iter().map(|(_, weight)| weight.abs()).collect();
        sizes.sort_by(f32::total_cmp);
        let threshold = sizes[sizes.len() / 2];
        assert!(sizes[0] < threshold);
        let heavy = train(
            &training,
            &TrainOptions {
                pair_threshold: f64::from(threshold),
                ..options(DCD, EXACT)
            },
        )
        .unwrap()
        .model;
        assert!(heavy.pair_count() < all.pair_count());
        assert!(heavy.pair_count() > 0);
        let PairWeights::Exact(pairs) = heavy.pairs() else {
            panic!("the pairs are exact");
        };
        assert!(pairs.iter().all(|(_, weight)| weight.abs() >= threshold));
        let PairWeights::Exact(pairs) = all.pairs() else {
            panic!("the pairs are exact");
        };
        assert!(pairs.iter().any(|(_, weight)| weight.abs() < threshold));
        // No pair without a weight is kept.
        assert!(pairs.iter().all(|(_, weight)| *weight != 0.0));
    }

    #[test]
    fn the_order_of_the_decisions_is_a_permutation_from_the_seed() {
        let mut list: Vec<usize> = (0..20).collect();
        let mut random = Random(1);
        random.shuffle(&mut list, 12);
        // The first 12 are mixed among themselves, the rest stays.
        assert_eq!(list[12..], [12, 13, 14, 15, 16, 17, 18, 19]);
        assert_ne!(list[..12], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let mut sorted = list.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<usize>>());
        // The same seed gives the same order, another seed another one.
        let mut again: Vec<usize> = (0..20).collect();
        Random(1).shuffle(&mut again, 12);
        assert_eq!(again, list);
        let mut other: Vec<usize> = (0..20).collect();
        Random(2).shuffle(&mut other, 12);
        assert_ne!(other, list);
        // SplitMix64: its first number from the seed 0.
        assert_eq!(Random(0).next(), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn a_training_that_cannot_run_is_refused() {
        let training = training_data();
        for cost in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            for algorithm in [
                Algorithm::DualCoordinateDescent { cost },
                Algorithm::PassiveAggressive {
                    cost,
                    iterations: 1,
                    averaged: false,
                },
            ] {
                assert_eq!(
                    refused(&training, &options(algorithm, EXACT)),
                    format!("the cost {cost} is not a number above 0")
                );
            }
        }
        let none = Algorithm::PassiveAggressive {
            cost: 1.0,
            iterations: 0,
            averaged: false,
        };
        assert_eq!(
            refused(&training, &options(none, EXACT)),
            "passive-aggressive needs an iteration"
        );
        for bits in [0, 31] {
            assert_eq!(
                refused(&training, &options(DCD, Pairs::Hashed { bits })),
                format!("the table of the pairs has 2^{bits} weights: 1 to 30 bits are allowed")
            );
        }
        for pair_threshold in [-0.5, f64::NAN] {
            let options = TrainOptions {
                pair_threshold,
                ..options(DCD, EXACT)
            };
            assert_eq!(
                refused(&training, &options),
                format!("the threshold of the pairs {pair_threshold} is not a number of 0 or more")
            );
        }

        // Sentences of two bunsetsu give no decision.
        let short = [sentence(&[("彼 は", Some(1)), ("読ん だ 。", None)])];
        assert_eq!(
            refused(&short, &options(DCD, EXACT)),
            "the training data has no decision: every sentence has two bunsetsu or fewer"
        );
        assert_eq!(
            refused(&[], &options(DCD, EXACT)),
            "the training data has no decision: every sentence has two bunsetsu or fewer"
        );

        // A sentence whose arcs cross, and one of another dictionary: the
        // message names the sentence.
        let mut crossing = training_data();
        crossing[3].bunsetsu[0].head = Some(2);
        crossing[3].bunsetsu[1].head = Some(3);
        let message = refused(&crossing, &options(DCD, EXACT));
        assert!(
            message.starts_with("sentence 4: invalid sentence: the parser cannot make the tree"),
            "{message}"
        );
        let mut narrow = training_data();
        narrow[1].morphemes[0] = LatticeMorpheme::new("彼", "名詞,代名詞");
        let message = refused(&narrow, &options(DCD, EXACT));
        assert!(
            message.starts_with("sentence 2: the morpheme 1 (\"彼\") has fewer than"),
            "{message}"
        );
    }

    #[test]
    fn the_defaults_are_those_chosen_on_ud_japanese_gsd() {
        let options = TrainOptions::default();
        assert_eq!(options.schema, Schema::ipadic());
        assert_eq!(options.features, FeatureSet::default());
        assert_eq!(
            options.algorithm,
            Algorithm::DualCoordinateDescent { cost: 0.0015 }
        );
        assert_eq!(options.pairs, Pairs::Hashed { bits: 22 });
        assert_eq!(options.min_count, 1);
        assert_eq!(options.pair_threshold, 0.0);
        assert_eq!(options.seed, 1);
    }
}

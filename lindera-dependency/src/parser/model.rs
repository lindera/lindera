//! The model of the parser: the weights of its classifier, and what makes
//! its features.
//!
//! [`ParserModel`] holds the weights in memory, as training gives them or as
//! they are made by hand. In a model file they are the parser part, which
//! [`ParserPart`] reads where it is.
//!
//! The weights are those of a bias, of the features and of pairs of
//! features. The weights of the pairs are kept in one of two ways
//! ([`PairWeights`]): each pair with a weight of its own, or a table of a
//! fixed size the pairs are hashed into.

use std::collections::HashMap;
use std::ops::Range;

use super::classifier::Classifier;
use super::features::FeatureSet;
use crate::bytes::{Reader, push_string, push_u32};
use crate::error::{Error, Result};
use crate::model::table::{self, Layout, Table};
use crate::schema::Schema;

/// The key of a feature for the table of hashed pairs: the hash of its
/// string, so that it does not depend on which other features a model has.
///
/// # Arguments
///
/// * `feature` - The feature string.
///
/// # Returns
///
/// The key: the 64-bit FNV-1a of the string, folded to 32 bits.
pub(crate) fn feature_key(feature: &str) -> u32 {
    table::hash(feature.as_bytes()) as u32
}

/// The place of a pair of features in a table of hashed pairs.
///
/// # Arguments
///
/// * `one` - The key of a feature ([`feature_key`]).
/// * `other` - The key of the other feature.
/// * `bits` - The size of the table, as a power of two: 1 to 30.
///
/// # Returns
///
/// The place, the same for the two orders of the features: the two keys,
/// the smaller first, mixed by the two multiplications of SplitMix64; the
/// highest `bits` bits of the result.
pub(crate) fn bucket(one: u32, other: u32, bits: u32) -> u32 {
    let mut value = (u64::from(one.min(other)) << 32) | u64::from(one.max(other));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value >> (64 - bits)) as u32
}

/// The weights of the pairs of features of a model.
#[derive(Debug, Clone, PartialEq)]
pub enum PairWeights {
    /// A weight of its own for each pair: the numbers of its two features,
    /// the smaller first, and the weight. The pairs are sorted, each once.
    Exact(Vec<((u32, u32), f32)>),
    /// A table of `2^bits` weights the pairs are hashed into; pairs that
    /// fall on the same weight share it.
    Hashed {
        /// The size of the table, as a power of two: 1 to 30.
        bits: u32,
        /// The weights, `2^bits` of them.
        weights: Vec<f32>,
    },
}

/// How the weights are written to a model file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantization {
    /// As they are: 4 bytes a weight.
    F32,
    /// As whole numbers of 2 bytes, times a scale: the largest weight
    /// becomes 32,767.
    I16,
    /// As whole numbers of 1 byte, times a scale: the largest weight
    /// becomes 127.
    I8,
}

impl Quantization {
    /// The three, with their names and their codes in a model file.
    const ALL: [(Quantization, &'static str, u8); 3] = [
        (Quantization::F32, "f32", 0),
        (Quantization::I16, "i16", 1),
        (Quantization::I8, "i8", 2),
    ];

    /// The name of the quantization: `f32`, `i16` or `i8`.
    ///
    /// # Returns
    ///
    /// The name.
    pub fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(one, _, _)| *one == self)
            .map_or("", |(_, name, _)| name)
    }

    /// The quantization with a name.
    ///
    /// # Arguments
    ///
    /// * `name` - `f32`, `i16` or `i8`.
    ///
    /// # Returns
    ///
    /// The quantization, or `None` for another name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, other, _)| *other == name)
            .map(|(one, _, _)| *one)
    }

    /// The code of the quantization in a model file.
    fn code(self) -> u8 {
        Self::ALL
            .iter()
            .find(|(one, _, _)| *one == self)
            .map_or(u8::MAX, |(_, _, code)| *code)
    }

    /// The quantization with a code.
    fn from_code(code: u8) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, _, other)| *other == code)
            .map(|(one, _, _)| *one)
    }

    /// The number of bytes of a weight.
    fn width(self) -> usize {
        match self {
            Quantization::F32 => 4,
            Quantization::I16 => 2,
            Quantization::I8 => 1,
        }
    }

    /// The largest whole number a weight is written as; `None` for weights
    /// written as they are.
    fn largest(self) -> Option<f64> {
        match self {
            Quantization::F32 => None,
            Quantization::I16 => Some(f64::from(i16::MAX)),
            Quantization::I8 => Some(f64::from(i8::MAX)),
        }
    }

    /// Appends a weight.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    /// * `weight` - The weight.
    /// * `scale` - What a whole number is multiplied by to give a weight.
    fn push(self, bytes: &mut Vec<u8>, weight: f32, scale: f64) {
        let whole = || (f64::from(weight) / scale).round();
        match self {
            Quantization::F32 => bytes.extend_from_slice(&weight.to_le_bytes()),
            Quantization::I16 => bytes.extend_from_slice(&(whole() as i16).to_le_bytes()),
            Quantization::I8 => bytes.extend_from_slice(&(whole() as i8).to_le_bytes()),
        }
    }

    /// Reads a weight in an array of them, before the scale.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The array.
    /// * `index` - The index of the weight.
    ///
    /// # Returns
    ///
    /// The weight as it is written; 0 past the end of the array.
    #[inline]
    fn read(self, bytes: &[u8], index: usize) -> f64 {
        match self {
            Quantization::F32 => {
                let start = index.saturating_mul(4);
                bytes
                    .get(start..start.saturating_add(4))
                    .and_then(|bytes| bytes.try_into().ok())
                    .map_or(0.0, |bytes| f64::from(f32::from_le_bytes(bytes)))
            }
            Quantization::I16 => {
                let start = index.saturating_mul(2);
                bytes
                    .get(start..start.saturating_add(2))
                    .and_then(|bytes| bytes.try_into().ok())
                    .map_or(0.0, |bytes| f64::from(i16::from_le_bytes(bytes)))
            }
            Quantization::I8 => bytes.get(index).map_or(0.0, |byte| f64::from(*byte as i8)),
        }
    }
}

/// The model of the parser, in memory.
///
/// It holds the weights of the classifier, and with them what makes the
/// feature strings they are for: the schema of the dictionary and the
/// feature set. It is what training gives. To keep it, put it into a model
/// file with [`ModelBuilder`](crate::model::ModelBuilder).
///
/// It is a [`Classifier`] itself, and scores a decision as the parser part
/// of a model file written without quantization does, to the last bit.
#[derive(Debug, Clone, PartialEq)]
pub struct ParserModel {
    /// The schema of the dictionary the model is for.
    schema: Schema,
    /// The groups of features the weights are for.
    feature_set: FeatureSet,
    /// The bias.
    bias: f64,
    /// The feature strings, sorted: the number of a feature is its index.
    names: Vec<String>,
    /// The weight of each feature.
    weights: Vec<f32>,
    /// The weights of the pairs.
    pairs: PairWeights,
    /// The number of each feature string.
    numbers: HashMap<String, u32>,
    /// The key of each feature for a table of hashed pairs.
    keys: Vec<u32>,
    /// The weight of each pair of an exact table, by the two numbers.
    exact: HashMap<(u32, u32), f32>,
}

impl ParserModel {
    /// Makes a model.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary the model is for.
    /// * `feature_set` - The groups of features the weights are for.
    /// * `bias` - The bias.
    /// * `features` - The feature strings with their weights, sorted by
    ///   the bytes of the strings, each once. The number of a feature is
    ///   its index.
    /// * `pairs` - The weights of the pairs.
    ///
    /// # Returns
    ///
    /// The model, or [`Error::Parser`] for a weight that is not finite,
    /// feature strings that are not sorted, pairs that are not sorted with
    /// the smaller number first or that name a feature that is not there,
    /// or a hashed table that has not `2^bits` weights with 1 to 30 bits.
    pub fn new(
        schema: Schema,
        feature_set: FeatureSet,
        bias: f64,
        features: Vec<(String, f32)>,
        pairs: PairWeights,
    ) -> Result<Self> {
        let finite = bias.is_finite()
            && features.iter().all(|(_, weight)| weight.is_finite())
            && match &pairs {
                PairWeights::Exact(pairs) => pairs.iter().all(|(_, weight)| weight.is_finite()),
                PairWeights::Hashed { weights, .. } => {
                    weights.iter().all(|weight| weight.is_finite())
                }
            };
        if !finite {
            return Err(Error::Parser("a weight is not finite".to_string()));
        }
        if features
            .windows(2)
            .any(|pair| pair[0].0.as_bytes() >= pair[1].0.as_bytes())
        {
            return Err(Error::Parser(
                "the features are not sorted, each once".to_string(),
            ));
        }
        if u32::try_from(features.len()).is_err() {
            return Err(Error::Parser("too many features".to_string()));
        }
        match &pairs {
            PairWeights::Exact(pairs) => {
                let count = features.len() as u32;
                if pairs
                    .iter()
                    .any(|((one, other), _)| one >= other || *other >= count)
                {
                    return Err(Error::Parser(
                        "a pair does not name two features, the smaller number first".to_string(),
                    ));
                }
                if pairs.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
                    return Err(Error::Parser(
                        "the pairs are not sorted, each once".to_string(),
                    ));
                }
            }
            PairWeights::Hashed { bits, weights } => {
                if !(1..=30).contains(bits) || weights.len() != 1usize << bits {
                    return Err(Error::Parser(format!(
                        "a table of hashed pairs of {bits} bits has {} weights",
                        weights.len()
                    )));
                }
            }
        }

        let (names, weights): (Vec<String>, Vec<f32>) = features.into_iter().unzip();
        let numbers = names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.clone(), index as u32))
            .collect();
        let keys = names.iter().map(|name| feature_key(name)).collect();
        let exact = match &pairs {
            PairWeights::Exact(pairs) => pairs.iter().copied().collect(),
            PairWeights::Hashed { .. } => HashMap::new(),
        };
        Ok(Self {
            schema,
            feature_set,
            bias,
            names,
            weights,
            pairs,
            numbers,
            keys,
            exact,
        })
    }

    /// The model without what has no weight: the pairs of an exact table
    /// whose weight is 0, and the features whose weight is 0 and that are
    /// in no pair that is kept. With a hashed table, a feature whose weight
    /// is 0 is taken out: training gives a feature the weight 0 only when
    /// no decision with it moved the weights.
    ///
    /// # Returns
    ///
    /// The smaller model.
    pub fn compact(self) -> Self {
        let pairs = match self.pairs {
            PairWeights::Exact(pairs) => PairWeights::Exact(
                pairs
                    .into_iter()
                    .filter(|(_, weight)| *weight != 0.0)
                    .collect(),
            ),
            hashed => hashed,
        };
        let mut kept: Vec<bool> = self.weights.iter().map(|weight| *weight != 0.0).collect();
        if let PairWeights::Exact(pairs) = &pairs {
            for ((one, other), _) in pairs {
                kept[*one as usize] = true;
                kept[*other as usize] = true;
            }
        }
        // The new number of each feature that is kept: the order stays.
        let mut renumbered = vec![u32::MAX; kept.len()];
        let mut next = 0;
        for (index, kept) in kept.iter().enumerate() {
            if *kept {
                renumbered[index] = next;
                next += 1;
            }
        }
        let features = self
            .names
            .into_iter()
            .zip(self.weights)
            .zip(&kept)
            .filter(|(_, kept)| **kept)
            .map(|(feature, _)| feature)
            .collect();
        let pairs = match pairs {
            PairWeights::Exact(pairs) => PairWeights::Exact(
                pairs
                    .into_iter()
                    .map(|((one, other), weight)| {
                        (
                            (renumbered[one as usize], renumbered[other as usize]),
                            weight,
                        )
                    })
                    .collect(),
            ),
            hashed => hashed,
        };
        // What was valid stays valid: the order of the features and of the
        // pairs is kept.
        Self::new(self.schema, self.feature_set, self.bias, features, pairs)
            .expect("a model without what has no weight is a model")
    }

    /// The schema of the dictionary the model is for.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The groups of features the weights are for.
    ///
    /// # Returns
    ///
    /// The feature set.
    pub fn feature_set(&self) -> &FeatureSet {
        &self.feature_set
    }

    /// The bias.
    ///
    /// # Returns
    ///
    /// The bias.
    pub fn bias(&self) -> f64 {
        self.bias
    }

    /// The number of features of the model.
    ///
    /// # Returns
    ///
    /// The number of feature strings.
    pub fn feature_count(&self) -> usize {
        self.names.len()
    }

    /// The weights of the pairs.
    ///
    /// # Returns
    ///
    /// The weights.
    pub fn pairs(&self) -> &PairWeights {
        &self.pairs
    }

    /// The number of weights of pairs that are not 0.
    ///
    /// # Returns
    ///
    /// The number of weights: of pairs for an exact table, of places for a
    /// hashed one.
    pub fn pair_count(&self) -> usize {
        match &self.pairs {
            PairWeights::Exact(pairs) => pairs.iter().filter(|(_, weight)| *weight != 0.0).count(),
            PairWeights::Hashed { weights, .. } => {
                weights.iter().filter(|weight| **weight != 0.0).count()
            }
        }
    }

    /// The scale of a quantization: what a whole number is multiplied by
    /// to give a weight.
    fn scale(&self, quantization: Quantization) -> f64 {
        let Some(largest) = quantization.largest() else {
            return 1.0;
        };
        let pairs: Box<dyn Iterator<Item = f32> + '_> = match &self.pairs {
            PairWeights::Exact(pairs) => Box::new(pairs.iter().map(|(_, weight)| *weight)),
            PairWeights::Hashed { weights, .. } => Box::new(weights.iter().copied()),
        };
        let highest = self
            .weights
            .iter()
            .copied()
            .chain(pairs)
            .fold(0.0f32, |highest, weight| highest.max(weight.abs()));
        if highest == 0.0 {
            1.0
        } else {
            f64::from(highest) / largest
        }
    }

    /// Appends the model to bytes, as the parser part of a model file.
    ///
    /// The feature set, the quantization, the bias, the scale, the table
    /// of the feature strings with their weights, and the weights of the
    /// pairs. The schema is not written: a model file holds it once. The
    /// same model gives the same bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    /// * `quantization` - How the weights are written.
    pub(crate) fn write_part(&self, bytes: &mut Vec<u8>, quantization: Quantization) {
        let scale = self.scale(quantization);
        push_string(bytes, &self.feature_set.to_string());
        bytes.push(quantization.code());
        bytes.push(match self.pairs {
            PairWeights::Exact(_) => PAIRS_EXACT,
            PairWeights::Hashed { .. } => PAIRS_HASHED,
        });
        bytes.extend_from_slice(&self.bias.to_le_bytes());
        bytes.extend_from_slice(&scale.to_le_bytes());

        // The value of a feature: its weight, and with hashed pairs its
        // key, so that a pair is hashed without reading a string.
        let hashed = matches!(self.pairs, PairWeights::Hashed { .. });
        let width = quantization.width() + if hashed { 4 } else { 0 };
        let values: Vec<Vec<u8>> = self
            .weights
            .iter()
            .zip(&self.keys)
            .map(|(weight, key)| {
                let mut value = Vec::with_capacity(width);
                quantization.push(&mut value, *weight, scale);
                if hashed {
                    value.extend_from_slice(&key.to_le_bytes());
                }
                value
            })
            .collect();
        let entries: Vec<(&str, &[u8])> = self
            .names
            .iter()
            .zip(&values)
            .map(|(name, value)| (name.as_str(), value.as_slice()))
            .collect();
        table::write(bytes, width, &entries);

        match &self.pairs {
            PairWeights::Exact(pairs) => {
                // The pairs by the smaller number: where the pairs of each
                // feature start, then the larger numbers, then the weights.
                bytes.extend_from_slice(&(pairs.len() as u64).to_le_bytes());
                let mut offsets = vec![0u32; self.names.len() + 1];
                for ((one, _), _) in pairs {
                    offsets[*one as usize + 1] += 1;
                }
                let mut total = 0;
                for offset in &mut offsets {
                    total += *offset;
                    *offset = total;
                }
                for offset in offsets {
                    push_u32(bytes, offset);
                }
                for ((_, other), _) in pairs {
                    push_u32(bytes, *other);
                }
                for (_, weight) in pairs {
                    quantization.push(bytes, *weight, scale);
                }
            }
            PairWeights::Hashed { bits, weights } => {
                push_u32(bytes, *bits);
                for weight in weights {
                    quantization.push(bytes, *weight, scale);
                }
            }
        }
    }
}

impl Classifier for ParserModel {
    fn id(&self, feature: &str) -> Option<u32> {
        self.numbers.get(feature).copied()
    }

    fn score(&self, features: &[u32]) -> f64 {
        // The sum of the weights first, then the bias, in the order the
        // parser part of a model file adds them.
        let mut sum = 0.0;
        for &feature in features {
            sum += self
                .weights
                .get(feature as usize)
                .map_or(0.0, |weight| f64::from(*weight));
        }
        match &self.pairs {
            PairWeights::Exact(_) => {
                for (index, &one) in features.iter().enumerate() {
                    for &other in &features[index + 1..] {
                        if let Some(weight) = self.exact.get(&(one, other)) {
                            sum += f64::from(*weight);
                        }
                    }
                }
            }
            PairWeights::Hashed { bits, weights } => {
                for (index, &one) in features.iter().enumerate() {
                    let Some(&one) = self.keys.get(one as usize) else {
                        continue;
                    };
                    for &other in &features[index + 1..] {
                        if let Some(&other) = self.keys.get(other as usize) {
                            sum += f64::from(weights[bucket(one, other, *bits) as usize]);
                        }
                    }
                }
            }
        }
        self.bias + sum
    }
}

/// The code of an exact table of pairs in a model file.
const PAIRS_EXACT: u8 = 0;
/// The code of a hashed table of pairs in a model file.
const PAIRS_HASHED: u8 = 1;

/// Where the weights of the pairs are in the bytes of a model.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PairsLayout {
    /// Each pair with a weight of its own, by the smaller number.
    Exact {
        /// Where the pairs of each feature start: one more than features.
        offsets: Range<usize>,
        /// The larger number of each pair.
        columns: Range<usize>,
        /// The weight of each pair.
        weights: Range<usize>,
    },
    /// A table the pairs are hashed into.
    Hashed {
        /// The size of the table, as a power of two.
        bits: u32,
        /// The weights.
        weights: Range<usize>,
    },
}

/// The parser part of a model: what is small is read, and the tables of
/// the weights are where they are in the bytes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParserLayout {
    /// The groups of features the weights are for.
    feature_set: FeatureSet,
    /// How the weights are written.
    quantization: Quantization,
    /// The bias.
    bias: f64,
    /// What a weight as it is written is multiplied by.
    scale: f64,
    /// The table of the feature strings.
    features: Layout,
    /// The weights of the pairs.
    pairs: PairsLayout,
}

/// A number of 4 bytes in an array of them; 0 past the end of the array.
#[inline]
fn read_u32(bytes: &[u8], index: usize) -> u32 {
    let start = index.saturating_mul(4);
    bytes
        .get(start..start.saturating_add(4))
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(0, u32::from_le_bytes)
}

impl ParserLayout {
    /// Reads and checks a parser part.
    ///
    /// # Arguments
    ///
    /// * `part` - The bytes of the part.
    /// * `base` - The position of the first byte of the part in the bytes
    ///   of the model.
    ///
    /// # Returns
    ///
    /// The layout, or [`Error::Model`] for bytes that are not a parser
    /// part: a feature set that cannot be read, an unknown quantization or
    /// table of pairs, a bias, a scale or a weight that is not finite, a
    /// scale that is not above 0, a
    /// table that is not one, pairs that are not sorted by their features,
    /// or bytes after the weights.
    pub(crate) fn read(part: &[u8], base: usize) -> Result<Self> {
        let mut reader = Reader::new(part, "model", Error::Model);
        let feature_set = FeatureSet::parse(&reader.string()?).map_err(|error| {
            reader.error(format_args!("has a feature set that is not one: {error}"))
        })?;
        let code = reader.u8()?;
        let quantization = Quantization::from_code(code)
            .ok_or_else(|| reader.error(format_args!("has the unknown quantization {code}")))?;
        let kind = reader.u8()?;
        let bias = reader.f64()?;
        let scale = reader.f64()?;
        if !(bias.is_finite() && scale.is_finite()) {
            return Err(reader.error("has a weight that is not finite"));
        }
        if scale <= 0.0 {
            return Err(reader.error(format_args!("has the scale {scale}, which is not above 0")));
        }
        let width = quantization.width();

        let hashed = match kind {
            PAIRS_EXACT => false,
            PAIRS_HASHED => true,
            _ => {
                return Err(reader.error(format_args!("has the unknown table of pairs {kind}")));
            }
        };
        let features = Layout::read(&mut reader, width + if hashed { 4 } else { 0 })?;
        let count = features.table(part).len();

        let (pairs, weights) = if hashed {
            let bits = reader.u32()?;
            if !(1..=30).contains(&bits) {
                return Err(
                    reader.error(format_args!("has a table of hashed pairs of {bits} bits"))
                );
            }
            let start = reader.position();
            let weights = reader.take((1usize << bits).saturating_mul(width))?;
            (
                PairsLayout::Hashed {
                    bits,
                    weights: start..reader.position(),
                },
                weights,
            )
        } else {
            let pair_count = usize::try_from(reader.u64()?)
                .map_err(|_| reader.error("has more pairs than can be read"))?;
            let start = reader.position();
            let offsets = reader.take((count + 1).saturating_mul(4))?;
            let offsets_range = start..reader.position();
            let start = reader.position();
            let columns = reader.take(pair_count.saturating_mul(4))?;
            let columns_range = start..reader.position();
            let start = reader.position();
            let weights = reader.take(pair_count.saturating_mul(width))?;
            let weights_range = start..reader.position();

            // The pairs of a feature follow those of the feature before,
            // and name larger features, in their order.
            let mut end = 0;
            for feature in 0..count {
                let from = read_u32(offsets, feature) as usize;
                let to = read_u32(offsets, feature + 1) as usize;
                // An offset past the pairs is refused before its pairs are
                // walked: bytes that claim a huge one do not keep the
                // reading busy.
                if from != end || to < from || to > pair_count {
                    return Err(
                        reader.error("has pairs that are not in the order of their features")
                    );
                }
                end = to;
                let mut before = feature as u32;
                for index in from..to {
                    let other = read_u32(columns, index);
                    if other <= before || other as usize >= count {
                        return Err(
                            reader.error("has pairs that are not in the order of their features")
                        );
                    }
                    before = other;
                }
            }
            if end != pair_count {
                return Err(reader.error("has pairs that are not in the order of their features"));
            }
            (
                PairsLayout::Exact {
                    offsets: offsets_range,
                    columns: columns_range,
                    weights: weights_range,
                },
                weights,
            )
        };
        if reader.remaining() != 0 {
            return Err(reader.error(format_args!(
                "has {} bytes after its parser",
                reader.remaining()
            )));
        }

        if quantization == Quantization::F32 {
            let table = features.table(part);
            let finite = (0..count)
                .all(|index| quantization.read(table.value(index), 0).is_finite())
                && (0..weights.len() / 4)
                    .all(|index| quantization.read(weights, index).is_finite());
            if !finite {
                return Err(reader.error("has a weight that is not finite"));
            }
        }

        let shift = |range: Range<usize>| base + range.start..base + range.end;
        Ok(Self {
            feature_set,
            quantization,
            bias,
            scale,
            features: features.shifted(base),
            pairs: match pairs {
                PairsLayout::Exact {
                    offsets,
                    columns,
                    weights,
                } => PairsLayout::Exact {
                    offsets: shift(offsets),
                    columns: shift(columns),
                    weights: shift(weights),
                },
                PairsLayout::Hashed { bits, weights } => PairsLayout::Hashed {
                    bits,
                    weights: shift(weights),
                },
            },
        })
    }

    /// The groups of features the weights are for.
    pub(crate) fn feature_set(&self) -> &FeatureSet {
        &self.feature_set
    }

    /// The part on the bytes of its model.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes of the model the layout was read from.
    ///
    /// # Returns
    ///
    /// The part.
    pub(crate) fn part<'a>(&'a self, bytes: &'a [u8]) -> ParserPart<'a> {
        let piece = |range: &Range<usize>| bytes.get(range.clone()).unwrap_or_default();
        ParserPart {
            layout: self,
            features: self.features.table(bytes),
            pairs: match &self.pairs {
                PairsLayout::Exact {
                    offsets,
                    columns,
                    weights,
                } => PairsPart::Exact {
                    offsets: piece(offsets),
                    columns: piece(columns),
                    weights: piece(weights),
                },
                PairsLayout::Hashed { bits, weights } => PairsPart::Hashed {
                    bits: *bits,
                    weights: piece(weights),
                },
            },
        }
    }
}

/// The weights of the pairs on the bytes of a model.
#[derive(Debug, Clone, Copy)]
enum PairsPart<'a> {
    /// Each pair with a weight of its own.
    Exact {
        /// Where the pairs of each feature start.
        offsets: &'a [u8],
        /// The larger number of each pair.
        columns: &'a [u8],
        /// The weight of each pair.
        weights: &'a [u8],
    },
    /// A table the pairs are hashed into.
    Hashed {
        /// The size of the table, as a power of two.
        bits: u32,
        /// The weights.
        weights: &'a [u8],
    },
}

/// The parser part of a model file, read on the bytes of the file: the
/// classifier of the parser that
/// [`Model::parser`](crate::model::Model::parser) gives.
#[derive(Debug, Clone, Copy)]
pub struct ParserPart<'a> {
    /// What is small of the part.
    layout: &'a ParserLayout,
    /// The table of the feature strings.
    features: Table<'a>,
    /// The weights of the pairs.
    pairs: PairsPart<'a>,
}

impl ParserPart<'_> {
    /// The number of features of the part.
    ///
    /// # Returns
    ///
    /// The number of feature strings.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// The number of weights of pairs of the part: of pairs for an exact
    /// table, of places for a hashed one.
    ///
    /// # Returns
    ///
    /// The number of weights, those that are 0 included.
    pub fn pair_count(&self) -> usize {
        match self.pairs {
            PairsPart::Exact { columns, .. } => columns.len() / 4,
            PairsPart::Hashed { bits, .. } => 1usize << bits,
        }
    }

    /// How the weights of the part are written.
    ///
    /// # Returns
    ///
    /// The quantization.
    pub fn quantization(&self) -> Quantization {
        self.layout.quantization
    }
}

impl Classifier for ParserPart<'_> {
    fn id(&self, feature: &str) -> Option<u32> {
        self.features.find(feature).map(|index| index as u32)
    }

    fn score(&self, features: &[u32]) -> f64 {
        let quantization = self.layout.quantization;
        let mut sum = 0.0;
        for &feature in features {
            sum += quantization.read(self.features.value(feature as usize), 0);
        }
        match self.pairs {
            PairsPart::Exact {
                offsets,
                columns,
                weights,
            } => {
                for (index, &one) in features.iter().enumerate() {
                    let mut from = read_u32(offsets, one as usize) as usize;
                    let to = read_u32(offsets, one as usize + 1) as usize;
                    // The other features are sorted, as the pairs of this
                    // one are: each is looked for after the one before.
                    for &other in &features[index + 1..] {
                        if from >= to {
                            break;
                        }
                        let (mut low, mut high) = (from, to);
                        while low < high {
                            let middle = low + (high - low) / 2;
                            if read_u32(columns, middle) < other {
                                low = middle + 1;
                            } else {
                                high = middle;
                            }
                        }
                        from = low;
                        if low < to && read_u32(columns, low) == other {
                            sum += quantization.read(weights, low);
                            from = low + 1;
                        }
                    }
                }
            }
            PairsPart::Hashed { bits, weights } => {
                let width = quantization.width();
                let key = |feature: u32| {
                    self.features
                        .value(feature as usize)
                        .get(width..width + 4)
                        .and_then(|bytes| bytes.try_into().ok())
                        .map(u32::from_le_bytes)
                };
                for (index, &one) in features.iter().enumerate() {
                    let Some(one) = key(one) else {
                        continue;
                    };
                    for &other in &features[index + 1..] {
                        if let Some(other) = key(other) {
                            sum += quantization.read(weights, bucket(one, other, bits) as usize);
                        }
                    }
                }
            }
        }
        self.layout.bias + self.layout.scale * sum
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The four features of the models of these tests, sorted.
    pub(crate) const NAMES: [&str; 4] = ["DIST:1", "SFS:は", "SFS:を", "a:が"];

    /// The features with their weights.
    fn features() -> Vec<(String, f32)> {
        NAMES
            .iter()
            .zip([0.5, -1.25, 2.0, 0.0])
            .map(|(name, weight)| (name.to_string(), weight))
            .collect()
    }

    /// A model with a weight of its own for three pairs.
    pub(crate) fn exact() -> ParserModel {
        ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            -1.0,
            features(),
            PairWeights::Exact(vec![((0, 2), 1.5), ((1, 3), -0.75), ((2, 3), 0.25)]),
        )
        .unwrap()
    }

    /// A model with a table of 16 weights the pairs are hashed into.
    pub(crate) fn hashed() -> ParserModel {
        ParserModel::new(
            Schema::ipadic(),
            FeatureSet::cabocha(),
            0.25,
            features(),
            PairWeights::Hashed {
                bits: 4,
                weights: (0..16).map(|index| index as f32 * 0.125 - 1.0).collect(),
            },
        )
        .unwrap()
    }

    /// The bytes of the parser part of a model.
    fn part(model: &ParserModel, quantization: Quantization) -> Vec<u8> {
        let mut part = Vec::new();
        model.write_part(&mut part, quantization);
        part
    }

    /// Every set of the four features, as sorted numbers.
    fn subsets() -> Vec<Vec<u32>> {
        (0..16u32)
            .map(|mask| (0..4).filter(|bit| mask & (1 << bit) != 0).collect())
            .collect()
    }

    /// The message of the error of bytes that are not a parser part.
    fn error(part: &[u8]) -> String {
        let error = ParserLayout::read(part, 0).unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        error.to_string()
    }

    /// The message of the error of what is not a model.
    fn refused(bias: f64, features: Vec<(String, f32)>, pairs: PairWeights) -> String {
        let error = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            bias,
            features,
            pairs,
        )
        .unwrap_err();
        assert!(matches!(error, Error::Parser(_)), "{error:?}");
        error.to_string()
    }

    #[test]
    fn a_model_scores_a_bias_its_features_and_their_pairs() {
        let model = exact();
        for (index, name) in NAMES.iter().enumerate() {
            assert_eq!(model.id(name), Some(index as u32));
        }
        assert_eq!(model.id("SFS:が"), None);
        assert_eq!(model.score(&[]), -1.0);
        assert_eq!(model.score(&[0]), -0.5);
        // The bias, 0.5 and 2.0, and the pair of the two, 1.5.
        assert_eq!(model.score(&[0, 2]), 3.0);
        assert_eq!(model.score(&[1, 3]), -3.0);
        assert_eq!(model.score(&[0, 1, 2, 3]), 1.25);
        // A number that is no feature adds nothing.
        assert_eq!(model.score(&[0, 9]), -0.5);
        assert_eq!(model.bias(), -1.0);
        assert_eq!(model.feature_count(), 4);
        assert_eq!(model.pair_count(), 3);
        assert_eq!(model.feature_set(), &FeatureSet::default());
        assert_eq!(model.schema(), &Schema::ipadic());
    }

    #[test]
    fn hashed_pairs_share_the_weight_of_their_place() {
        let model = hashed();
        let keys: Vec<u32> = NAMES.iter().map(|name| feature_key(name)).collect();
        let PairWeights::Hashed { bits, weights } = model.pairs() else {
            panic!("the pairs are hashed");
        };
        let weights = weights.clone();
        let place = |one: usize, other: usize| bucket(keys[one], keys[other], *bits) as usize;
        assert_eq!(
            model.score(&[0, 2]),
            0.25 + 0.5 + 2.0 + f64::from(weights[place(0, 2)])
        );
        let all: f64 = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
            .iter()
            .map(|(one, other)| f64::from(weights[place(*one, *other)]))
            .sum();
        assert_eq!(model.score(&[0, 1, 2, 3]), 0.25 + (1.25 + all));
        // 15 of the 16 places have a weight that is not 0.
        assert_eq!(model.pair_count(), 15);
    }

    #[test]
    fn the_place_of_a_pair_is_the_same_for_its_two_orders() {
        // The key of a feature is the hash of the tables: of `a`, the
        // 64-bit FNV-1a af63dc4c8601ec8c folded to 32 bits.
        assert_eq!(feature_key("a"), 0xaf63_dc4c ^ 0x8601_ec8c);
        let (one, other) = (feature_key("SFS:は"), feature_key("DIST:1"));
        for bits in [1, 4, 22, 30] {
            let place = bucket(one, other, bits);
            assert_eq!(place, bucket(other, one, bits));
            assert!(place < 1 << bits);
        }
        // More bits keep the first ones: a table twice as large splits
        // each place in two.
        assert_eq!(bucket(one, other, 22) >> 2, bucket(one, other, 20));
        // The two keys, the smaller first, mixed by two multiplications.
        let mut value = (u64::from(one.min(other)) << 32) | u64::from(one.max(other));
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        assert_eq!(bucket(one, other, 30), (value >> 34) as u32);
        // The place of two keys, as a number to hold the format to.
        assert_eq!(bucket(1, 2, 16), {
            let mut value = (1u64 << 32) | 2;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            (value >> 48) as u32
        });
        // Pairs of the four features do not all fall on one place.
        let keys: Vec<u32> = NAMES.iter().map(|name| feature_key(name)).collect();
        let mut places: Vec<u32> = vec![
            bucket(keys[0], keys[1], 4),
            bucket(keys[0], keys[2], 4),
            bucket(keys[1], keys[2], 4),
            bucket(keys[2], keys[3], 4),
        ];
        places.dedup();
        assert!(places.len() > 1);
    }

    #[test]
    fn what_is_not_a_model_is_refused() {
        let pairs = || PairWeights::Exact(vec![((0, 2), 1.5)]);
        assert_eq!(
            refused(f64::NAN, features(), pairs()),
            "a weight is not finite"
        );
        let mut other = features();
        other[1].1 = f32::INFINITY;
        assert_eq!(refused(0.0, other, pairs()), "a weight is not finite");
        assert_eq!(
            refused(
                0.0,
                features(),
                PairWeights::Exact(vec![((0, 2), f32::NAN)])
            ),
            "a weight is not finite"
        );
        let table = |weight: f32| PairWeights::Hashed {
            bits: 1,
            weights: vec![0.0, weight],
        };
        assert_eq!(
            refused(0.0, features(), table(f32::NEG_INFINITY)),
            "a weight is not finite"
        );
        assert!(
            ParserModel::new(
                Schema::ipadic(),
                FeatureSet::default(),
                0.0,
                features(),
                table(1.0)
            )
            .is_ok()
        );

        // Features in another order, and a feature twice.
        for (one, other) in [(0, 1), (1, 1)] {
            let mut unsorted = features();
            unsorted[one].0 = features()[other].0.clone();
            unsorted[other].0 = features()[one].0.clone();
            if one == other {
                unsorted[0].0 = unsorted[1].0.clone();
            }
            assert_eq!(
                refused(0.0, unsorted, pairs()),
                "the features are not sorted, each once"
            );
        }

        // A pair of a feature with itself, the larger number first, and a
        // feature that is not there.
        for pair in [(1, 1), (2, 0), (0, 4)] {
            assert_eq!(
                refused(0.0, features(), PairWeights::Exact(vec![(pair, 1.0)])),
                "a pair does not name two features, the smaller number first"
            );
        }
        // Pairs in another order, and a pair twice.
        for pairs in [[(1, 3), (0, 2)], [(0, 2), (0, 2)]] {
            let pairs = pairs.iter().map(|pair| (*pair, 1.0)).collect();
            assert_eq!(
                refused(0.0, features(), PairWeights::Exact(pairs)),
                "the pairs are not sorted, each once"
            );
        }

        // A table that has not 2^bits weights, and no bits or too many.
        for (bits, count) in [(2, 3), (2, 5), (0, 1), (31, 2)] {
            let table = PairWeights::Hashed {
                bits,
                weights: vec![0.0; count],
            };
            assert_eq!(
                refused(0.0, features(), table),
                format!("a table of hashed pairs of {bits} bits has {count} weights")
            );
        }
    }

    #[test]
    fn a_compact_model_has_only_what_has_a_weight() {
        // Of five features, `b` has no weight and is in no pair with one,
        // and `d` has no weight but is in a pair.
        let features: Vec<(String, f32)> = ["a", "b", "c", "d", "e"]
            .iter()
            .zip([1.0, 0.0, -2.0, 0.0, 0.5])
            .map(|(name, weight)| (name.to_string(), weight))
            .collect();
        let model = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.5,
            features.clone(),
            PairWeights::Exact(vec![((0, 1), 0.0), ((0, 4), 3.0), ((2, 3), -1.0)]),
        )
        .unwrap();
        let before: Vec<f64> = [vec![0, 4], vec![2, 3], vec![0, 1, 2, 3, 4]]
            .iter()
            .map(|features| model.score(features))
            .collect();
        let compact = model.compact();
        assert_eq!(compact.feature_count(), 4);
        assert_eq!(compact.id("b"), None);
        // The numbers follow the order that is left: a, c, d, e.
        assert_eq!(compact.id("c"), Some(1));
        assert_eq!(compact.id("d"), Some(2));
        assert_eq!(
            compact.pairs(),
            &PairWeights::Exact(vec![((0, 3), 3.0), ((1, 2), -1.0)])
        );
        let after: Vec<f64> = [vec![0, 3], vec![1, 2], vec![0, 1, 2, 3]]
            .iter()
            .map(|features| compact.score(features))
            .collect();
        assert_eq!(after, before);
        // Nothing more is taken out of it.
        assert_eq!(compact.clone().compact(), compact);

        // With hashed pairs, a feature without a weight is taken out, and
        // the table stays.
        let table = PairWeights::Hashed {
            bits: 2,
            weights: vec![0.0, 1.0, 0.0, -1.0],
        };
        let model = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.5,
            features,
            table.clone(),
        )
        .unwrap()
        .compact();
        assert_eq!(model.feature_count(), 3);
        assert_eq!(model.id("d"), None);
        assert_eq!(model.pairs(), &table);
    }

    #[test]
    fn a_quantization_has_a_name_and_a_code() {
        for (quantization, name, code) in Quantization::ALL {
            assert_eq!(quantization.name(), name);
            assert_eq!(Quantization::from_name(name), Some(quantization));
            assert_eq!(quantization.code(), code);
            assert_eq!(Quantization::from_code(code), Some(quantization));
        }
        // The names are those of the command, and the codes those of a
        // model file.
        assert_eq!(Quantization::from_name("f32"), Some(Quantization::F32));
        assert_eq!(Quantization::from_name("i16"), Some(Quantization::I16));
        assert_eq!(Quantization::from_name("i8"), Some(Quantization::I8));
        assert_eq!(
            [
                Quantization::F32.code(),
                Quantization::I16.code(),
                Quantization::I8.code()
            ],
            [0, 1, 2]
        );
        assert_eq!(Quantization::from_name("f64"), None);
        assert_eq!(Quantization::from_code(3), None);
        assert_eq!(
            [
                Quantization::F32.width(),
                Quantization::I16.width(),
                Quantization::I8.width()
            ],
            [4, 2, 1]
        );
    }

    #[test]
    fn the_part_of_a_file_scores_as_the_model_in_memory() {
        for model in [exact(), hashed()] {
            let bytes = part(&model, Quantization::F32);
            let layout = ParserLayout::read(&bytes, 0).unwrap();
            let read = layout.part(&bytes);
            assert_eq!(layout.feature_set(), model.feature_set());
            assert_eq!(read.feature_count(), 4);
            assert_eq!(read.quantization(), Quantization::F32);
            for (index, name) in NAMES.iter().enumerate() {
                assert_eq!(read.id(name), Some(index as u32));
            }
            assert_eq!(read.id("SFS:が"), None);
            // Without quantization, to the last bit.
            for features in subsets() {
                assert_eq!(
                    read.score(&features).to_bits(),
                    model.score(&features).to_bits(),
                    "{features:?}"
                );
            }
            // The same model gives the same bytes.
            assert_eq!(part(&model.clone(), Quantization::F32), bytes);
        }
        let bytes = part(&exact(), Quantization::F32);
        assert_eq!(
            ParserLayout::read(&bytes, 0)
                .unwrap()
                .part(&bytes)
                .pair_count(),
            3
        );
        let bytes = part(&hashed(), Quantization::F32);
        assert_eq!(
            ParserLayout::read(&bytes, 0)
                .unwrap()
                .part(&bytes)
                .pair_count(),
            16
        );
    }

    #[test]
    fn quantized_weights_are_whole_numbers_times_a_scale() {
        for model in [exact(), hashed()] {
            let sizes: Vec<usize> = Quantization::ALL
                .iter()
                .map(|(quantization, _, _)| part(&model, *quantization).len())
                .collect();
            assert!(sizes[0] > sizes[1] && sizes[1] > sizes[2], "{sizes:?}");

            for (quantization, largest) in [(Quantization::I16, 32767.0), (Quantization::I8, 127.0)]
            {
                let bytes = part(&model, quantization);
                let layout = ParserLayout::read(&bytes, 0).unwrap();
                let read = layout.part(&bytes);
                assert_eq!(read.quantization(), quantization);
                // The largest weight of both models is 2.0.
                let scale = 2.0 / largest;
                assert_eq!(layout.scale, scale);
                for features in subsets() {
                    // A weight is off by half a step at most: the four
                    // features and their six pairs.
                    let weights =
                        features.len() + features.len() * features.len().saturating_sub(1) / 2;
                    let difference = (read.score(&features) - model.score(&features)).abs();
                    assert!(
                        difference <= scale / 2.0 * weights as f64 + 1e-12,
                        "{quantization:?} {features:?}: {difference}"
                    );
                }
                // The bias is not quantized.
                assert_eq!(read.score(&[]), model.bias());
            }
        }
        // The largest weight becomes the largest whole number: 2.0 of
        // `SFS:を` is 127, and -1.25 of `SFS:は` is -79 (-79.4).
        let bytes = part(&exact(), Quantization::I8);
        let read = ParserLayout::read(&bytes, 0).unwrap();
        let read = read.part(&bytes);
        assert_eq!(read.score(&[2]), -1.0 + 2.0 / 127.0 * 127.0);
        assert_eq!(read.score(&[1]), -1.0 + 2.0 / 127.0 * -79.0);
        // 0.5 of `DIST:1` is 31.75 steps, and is written as 32: the nearest
        // whole number, not the one towards 0.
        assert_eq!(read.score(&[0]), -1.0 + 2.0 / 127.0 * 32.0);
        // The scale is from the largest weight of all, a pair as well: here
        // the pair of 4.0, where the largest feature has 2.0.
        let heavy = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.0,
            features(),
            PairWeights::Exact(vec![((0, 1), -4.0)]),
        )
        .unwrap();
        let bytes = part(&heavy, Quantization::I8);
        assert_eq!(ParserLayout::read(&bytes, 0).unwrap().scale, 4.0 / 127.0);
        let table = PairWeights::Hashed {
            bits: 1,
            weights: vec![0.0, 8.0],
        };
        let heavy = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.0,
            features(),
            table,
        )
        .unwrap();
        let bytes = part(&heavy, Quantization::I16);
        assert_eq!(ParserLayout::read(&bytes, 0).unwrap().scale, 8.0 / 32767.0);
        // A model without a weight has the scale 1.
        let empty = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.5,
            Vec::new(),
            PairWeights::Exact(Vec::new()),
        )
        .unwrap();
        let bytes = part(&empty, Quantization::I8);
        let layout = ParserLayout::read(&bytes, 0).unwrap();
        assert_eq!(layout.scale, 1.0);
        assert_eq!(layout.part(&bytes).score(&[]), 0.5);
        assert_eq!(layout.part(&bytes).id("a"), None);
    }

    /// A model of 40 features, with weights from a fixed sequence: every
    /// third pair has a weight in the exact table.
    fn larger(hashed: bool) -> ParserModel {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            // Xorshift: any fixed sequence will do.
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state % 2001) as f32 - 1000.0) / 250.0
        };
        let features: Vec<(String, f32)> = (0..40)
            .map(|index| (format!("F{index:02}:素性"), next()))
            .collect();
        let pairs = if hashed {
            PairWeights::Hashed {
                bits: 7,
                weights: (0..128).map(|_| next()).collect(),
            }
        } else {
            let mut pairs = Vec::new();
            for one in 0..40u32 {
                for other in one + 1..40 {
                    if (one * 7 + other) % 3 == 0 {
                        pairs.push(((one, other), next()));
                    }
                }
            }
            PairWeights::Exact(pairs)
        };
        ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.125,
            features,
            pairs,
        )
        .unwrap()
    }

    #[test]
    fn a_larger_model_scores_the_same_from_a_file() {
        for hashed in [false, true] {
            let model = larger(hashed);
            let bytes = part(&model, Quantization::F32);
            let layout = ParserLayout::read(&bytes, 0).unwrap();
            let read = layout.part(&bytes);
            assert_eq!(read.feature_count(), 40);
            // Sets of features of every size, spread over the 40: the
            // feature `index` is in the set `mask` when a bit of the mask
            // says so.
            let mut different = 0;
            for mask in 1..400u64 {
                let features: Vec<u32> = (0..40u32)
                    .filter(|index| (mask.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> index) & 1 == 1)
                    .collect();
                let (from_file, in_memory) = (read.score(&features), model.score(&features));
                assert_eq!(from_file.to_bits(), in_memory.to_bits(), "{features:?}");
                different += usize::from(in_memory != model.bias());
            }
            assert!(different > 390);
            // All of them, and each alone.
            let all: Vec<u32> = (0..40).collect();
            assert_eq!(read.score(&all).to_bits(), model.score(&all).to_bits());
            for feature in 0..40 {
                assert_eq!(read.score(&[feature]), model.score(&[feature]));
                assert_eq!(read.id(&format!("F{feature:02}:素性")), Some(feature));
            }
        }
    }

    #[test]
    fn the_layout_is_counted_from_the_base() {
        for model in [exact(), hashed()] {
            let part = part(&model, Quantization::I16);
            let mut bytes = vec![0xa5; 13];
            bytes.extend_from_slice(&part);
            let layout = ParserLayout::read(&part, 13).unwrap();
            let read = layout.part(&bytes);
            let alone = ParserLayout::read(&part, 0).unwrap();
            for features in subsets() {
                assert_eq!(read.score(&features), alone.part(&part).score(&features));
            }
            assert_eq!(read.id("SFS:を"), Some(2));
            // On other bytes it finds nothing, and does not panic.
            let read = layout.part(&bytes[..20]);
            assert_eq!(read.id("SFS:を"), None);
            let _ = read.score(&[0, 1, 2, 3]);
        }
    }

    /// The position of the quantization in a part with the default feature
    /// set: after the length and the text of the set.
    fn after_the_set(model: &ParserModel) -> usize {
        4 + model.feature_set().to_string().len()
    }

    #[test]
    fn refuses_a_parser_part_that_is_not_one() {
        for model in [exact(), hashed()] {
            let bytes = part(&model, Quantization::F32);
            assert!(ParserLayout::read(&bytes, 0).is_ok());
            let start = after_the_set(&model);

            // The feature set is the text of its groups.
            let mut other = bytes.clone();
            other[4] = b'x';
            assert!(
                error(&other).starts_with("the model has a feature set that is not one: "),
                "{}",
                error(&other)
            );

            let mut other = bytes.clone();
            other[start] = 3;
            assert_eq!(error(&other), "the model has the unknown quantization 3");
            let mut other = bytes.clone();
            other[start + 1] = 2;
            assert_eq!(error(&other), "the model has the unknown table of pairs 2");

            // The bias and the scale are the 16 bytes after the two codes.
            for offset in [start + 2, start + 10] {
                let mut other = bytes.clone();
                other[offset..offset + 8].copy_from_slice(&f64::NAN.to_le_bytes());
                assert_eq!(error(&other), "the model has a weight that is not finite");
            }

            // A scale of 0, and one below.
            for scale in [0.0f64, -1.0] {
                let mut other = bytes.clone();
                other[start + 10..start + 18].copy_from_slice(&scale.to_le_bytes());
                assert_eq!(
                    error(&other),
                    format!("the model has the scale {scale}, which is not above 0")
                );
            }

            // The last weight: of a pair, or of a place.
            let mut other = bytes.clone();
            let last = other.len() - 4;
            other[last..].copy_from_slice(&f32::INFINITY.to_le_bytes());
            assert_eq!(error(&other), "the model has a weight that is not finite");

            let mut other = bytes.clone();
            other.push(0);
            assert_eq!(error(&other), "the model has 1 bytes after its parser");
            for length in 0..bytes.len() {
                let message = error(&bytes[..length]);
                assert!(
                    message.starts_with("the model ends early"),
                    "{length}: {message}"
                );
            }
        }
    }

    #[test]
    fn refuses_weights_of_features_and_pairs_that_are_not_what_the_format_says() {
        // The weight of a feature that is not a number: the values of the
        // table of the features are before its 8 slots, and the weights of
        // the 3 pairs, their columns and the 5 offsets after it.
        let bytes = part(&exact(), Quantization::F32);
        let pairs = bytes.len() - (8 + 5 * 4 + 3 * 4 + 3 * 4);
        let values = pairs - 8 * 4 - 4 * 4;
        assert_eq!(bytes[values..values + 4], 0.5f32.to_le_bytes());
        let mut other = bytes.clone();
        other[values..values + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert_eq!(error(&other), "the model has a weight that is not finite");

        // The pairs: their number, the offsets of the four features and
        // the end, the larger numbers, the weights.
        assert_eq!(bytes[pairs..pairs + 8], 3u64.to_le_bytes());
        let offsets = pairs + 8;
        let offset = |index: usize| bytes[offsets + 4 * index];
        assert_eq!(
            [offset(0), offset(1), offset(2), offset(3), offset(4)],
            [0, 1, 2, 3, 3]
        );
        let columns = offsets + 5 * 4;
        assert_eq!(
            [bytes[columns], bytes[columns + 4], bytes[columns + 8]],
            [2, 3, 3]
        );
        let order = "the model has pairs that are not in the order of their features";
        // The first offset is not 0; an offset goes back; the last is not
        // the number of pairs.
        for (index, value) in [(0, 1), (2, 0), (4, 2), (4, 4)] {
            let mut other = bytes.clone();
            other[offsets + 4 * index] = value;
            assert_eq!(error(&other), order, "offset {index} = {value}");
        }
        // The pair of the feature 0 names itself, a smaller feature would
        // be the same; the pair of 1 names a feature that is not there.
        for (index, value) in [(0, 0), (1, 4), (2, 2)] {
            let mut other = bytes.clone();
            other[columns + 4 * index] = value;
            assert_eq!(error(&other), order, "column {index} = {value}");
        }
        // Two pairs of one feature in the wrong order.
        let model = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.0,
            features(),
            PairWeights::Exact(vec![((0, 1), 1.0), ((0, 2), 1.0)]),
        )
        .unwrap();
        let bytes = part(&model, Quantization::F32);
        let columns = bytes.len() - 2 * 4 - 2 * 4;
        assert_eq!([bytes[columns], bytes[columns + 4]], [1, 2]);
        let mut other = bytes.clone();
        other[columns] = 2;
        other[columns + 4] = 1;
        assert_eq!(error(&other), order);
        let mut other = bytes.clone();
        other[columns + 4] = 1;
        assert_eq!(error(&other), order);
        // Offsets that go back, with pairs that are in order all the same:
        // of five features, the pairs of the third start before the end of
        // those of the first.
        let five: Vec<(String, f32)> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|name| (name.to_string(), 1.0))
            .collect();
        let model = ParserModel::new(
            Schema::ipadic(),
            FeatureSet::default(),
            0.0,
            five,
            PairWeights::Exact(vec![((0, 1), 1.0), ((0, 3), 1.0), ((2, 4), 1.0)]),
        )
        .unwrap();
        let bytes = part(&model, Quantization::F32);
        let columns = bytes.len() - 3 * 4 - 3 * 4;
        let offsets = columns - 6 * 4;
        let offset = |index: usize| bytes[offsets + 4 * index];
        assert_eq!(
            [
                offset(0),
                offset(1),
                offset(2),
                offset(3),
                offset(4),
                offset(5)
            ],
            [0, 2, 2, 3, 3, 3]
        );
        assert_eq!(
            [bytes[columns], bytes[columns + 4], bytes[columns + 8]],
            [1, 3, 4]
        );
        let mut other = bytes.clone();
        other[offsets + 4 * 2] = 1;
        assert_eq!(error(&other), order);
        // Offsets that end before the last pair: a pair of no feature.
        let mut other = bytes.clone();
        other[offsets + 4 * 3] = 2;
        other[offsets + 4 * 4] = 2;
        other[offsets + 4 * 5] = 2;
        assert_eq!(error(&other), order);

        // More pairs than the bytes hold.
        let pairs = columns - 5 * 4 - 8;
        let mut other = bytes.clone();
        other[pairs..pairs + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(error(&other).starts_with("the model ends early"));

        // A table of hashed pairs of no bit, and of too many.
        let bytes = part(&hashed(), Quantization::I8);
        let bits = bytes.len() - 16 - 4;
        assert_eq!(bytes[bits..bits + 4], 4u32.to_le_bytes());
        for value in [0, 31] {
            let mut other = bytes.clone();
            other[bits] = value;
            assert_eq!(
                error(&other),
                format!("the model has a table of hashed pairs of {value} bits")
            );
        }
        let mut other = bytes.clone();
        other[bits] = 5;
        assert!(error(&other).starts_with("the model ends early"));
    }

    #[test]
    fn no_changed_byte_makes_reading_or_scoring_panic() {
        for (model, quantization) in [
            (exact(), Quantization::F32),
            (exact(), Quantization::I8),
            (hashed(), Quantization::I16),
        ] {
            let bytes = part(&model, quantization);
            let mut read = 0;
            for position in 0..bytes.len() {
                for value in [0, 1, 0x7f, 0x80, 0xff, bytes[position] ^ 1] {
                    if value == bytes[position] {
                        continue;
                    }
                    let mut other = bytes.clone();
                    other[position] = value;
                    if let Ok(layout) = ParserLayout::read(&other, 0) {
                        read += 1;
                        let part = layout.part(&other);
                        for name in NAMES {
                            let _ = part.id(name);
                        }
                        let _ = part.score(&[0, 1, 2, 3]);
                        let _ = part.score(&[3, 2, 1, 0, 9, 9]);
                    }
                }
            }
            assert!(read > 0);
        }
    }
}

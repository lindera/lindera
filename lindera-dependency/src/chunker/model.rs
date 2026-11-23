//! The model of the chunker: its weights, and what makes its features.

use std::collections::HashMap;

use super::columns::Column;
use super::template::Template;
use crate::bytes::{Reader, push_string, push_u32};
use crate::error::{Error, Result};
use crate::schema::Schema;

/// The first bytes of a model.
const MAGIC: &[u8; 8] = b"LDCHUNK\0";

/// The version of the bytes of a model. Version 2 holds the whole schema,
/// with its patterns and its symbols; version 1 held its columns only.
const VERSION: u32 = 2;

/// A label of the chunker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The morpheme starts a bunsetsu.
    Begin,
    /// The morpheme continues the bunsetsu of the one before it.
    Inside,
}

impl Label {
    /// The two labels, in the order of their weights.
    pub(crate) const ALL: [Label; 2] = [Label::Begin, Label::Inside];

    /// The index of the weight of the label in the weights of a feature.
    ///
    /// # Returns
    ///
    /// 0 for [`Label::Begin`], 1 for [`Label::Inside`].
    pub(crate) fn index(self) -> usize {
        match self {
            Label::Begin => 0,
            Label::Inside => 1,
        }
    }
}

/// The index of the start and of the end of a sentence in the transitions.
pub(crate) const BOUNDARY: usize = 0;

/// The model of the chunker.
///
/// It holds the weights, and with them what makes the feature strings they
/// are for: the schema of the dictionary, the columns and the template.
/// Training and inference then cannot disagree on the features.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkerModel {
    /// The schema of the dictionary the model is for.
    schema: Schema,
    /// The columns the template reads.
    columns: Vec<Column>,
    /// The template.
    template: Template,
    /// The weights of the transitions, as `[from][to]`: 0 is the start of
    /// the sentence as `from` and its end as `to`, 1 is `B`, 2 is `I`.
    transitions: [[f64; 3]; 3],
    /// The weights of the feature strings: of `B`, then of `I`.
    features: HashMap<String, [f64; 2]>,
}

impl ChunkerModel {
    /// Makes a model.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary the model is for.
    /// * `columns` - The columns the template reads, in the order of their
    ///   numbers in the template.
    /// * `template` - The template.
    /// * `transitions` - The weights of the transitions, as `[from][to]`: 0
    ///   is the start of the sentence as `from` and its end as `to`, 1 is
    ///   `B`, 2 is `I`.
    /// * `features` - The weights of the feature strings: of `B`, then of
    ///   `I`.
    ///
    /// # Returns
    ///
    /// The model, or [`Error::Chunker`] when the template reads a column
    /// that `columns` does not have, or a weight is not finite.
    pub fn new(
        schema: Schema,
        columns: Vec<Column>,
        template: Template,
        transitions: [[f64; 3]; 3],
        features: HashMap<String, [f64; 2]>,
    ) -> Result<Self> {
        if template.columns() > columns.len() {
            return Err(Error::Chunker(format!(
                "the template reads {} columns, and {} are given",
                template.columns(),
                columns.len()
            )));
        }
        let finite = transitions
            .iter()
            .flatten()
            .all(|weight| weight.is_finite())
            && features.values().flatten().all(|weight| weight.is_finite());
        if !finite {
            return Err(Error::Chunker("a weight is not finite".to_string()));
        }
        Ok(Self {
            schema,
            columns,
            template,
            transitions,
            features,
        })
    }

    /// The schema of the dictionary the model is for.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The columns the template reads.
    ///
    /// # Returns
    ///
    /// The columns.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The template.
    ///
    /// # Returns
    ///
    /// The template.
    pub fn template(&self) -> &Template {
        &self.template
    }

    /// The weight of a transition.
    ///
    /// # Arguments
    ///
    /// * `from` - 0 for the start of the sentence, 1 for `B`, 2 for `I`.
    /// * `to` - 0 for the end of the sentence, 1 for `B`, 2 for `I`.
    ///
    /// # Returns
    ///
    /// The weight; 0 for an index over 2.
    pub fn transition(&self, from: usize, to: usize) -> f64 {
        self.transitions
            .get(from)
            .and_then(|row| row.get(to))
            .copied()
            .unwrap_or(0.0)
    }

    /// The weights of a feature string.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string, as the template expands it.
    ///
    /// # Returns
    ///
    /// The weight of `B` and the weight of `I`, or `None` for a string the
    /// model does not have.
    pub fn feature(&self, feature: &str) -> Option<[f64; 2]> {
        self.features.get(feature).copied()
    }

    /// The number of feature strings of the model.
    ///
    /// # Returns
    ///
    /// The number of strings.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// The number of weights of the model that are not 0: those of the
    /// feature strings and those of the transitions.
    ///
    /// # Returns
    ///
    /// The number of weights.
    pub fn weight_count(&self) -> usize {
        self.features
            .values()
            .flatten()
            .chain(self.transitions.iter().flatten())
            .filter(|weight| **weight != 0.0)
            .count()
    }

    /// Writes the model as bytes.
    ///
    /// The format is the one of this crate at this version, little endian:
    /// a magic number, the version, the schema, the columns, the template,
    /// the transitions, and the feature strings in order with their
    /// weights. The same model gives the same bytes. It is not the file
    /// format of a released model.
    ///
    /// # Returns
    ///
    /// The bytes, which [`ChunkerModel::from_bytes`] reads.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        push_u32(&mut bytes, VERSION);

        self.schema.write(&mut bytes);

        push_u32(&mut bytes, self.columns.len() as u32);
        for column in &self.columns {
            bytes.push(column.code());
        }

        push_string(&mut bytes, &self.template.to_string());

        for weight in self.transitions.iter().flatten() {
            bytes.extend_from_slice(&weight.to_le_bytes());
        }

        let mut features: Vec<(&String, &[f64; 2])> = self.features.iter().collect();
        features.sort_by(|left, right| left.0.cmp(right.0));
        bytes.extend_from_slice(&(features.len() as u64).to_le_bytes());
        for (feature, weights) in features {
            push_string(&mut bytes, feature);
            for weight in weights {
                bytes.extend_from_slice(&weight.to_le_bytes());
            }
        }
        bytes
    }

    /// Reads a model from bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes [`ChunkerModel::to_bytes`] wrote.
    ///
    /// # Returns
    ///
    /// The model, or [`Error::Chunker`] for bytes that are not a model of
    /// this version: another magic number or version, bytes that end early
    /// or go on after the model, or content [`ChunkerModel::new`] refuses.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes, "chunker model", Error::Chunker);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(Error::Chunker(
                "not a chunker model: the magic number is missing".to_string(),
            ));
        }
        let version = reader.u32()?;
        if version != VERSION {
            return Err(Error::Chunker(format!(
                "the chunker model has the version {version}, and this crate reads {VERSION}"
            )));
        }

        let schema = Schema::read(&mut reader)?;

        let count = reader.u32()? as usize;
        let mut columns = Vec::new();
        for &code in reader.take(count)? {
            columns.push(Column::from_code(code).ok_or_else(|| {
                Error::Chunker(format!("the chunker model has the unknown column {code}"))
            })?);
        }

        let template = Template::parse(&reader.string()?)?;

        let mut transitions = [[0.0; 3]; 3];
        for weight in transitions.iter_mut().flatten() {
            *weight = reader.f64()?;
        }

        let count = reader.u64()?;
        let mut features = HashMap::new();
        for _ in 0..count {
            let feature = reader.string()?;
            let weights = [reader.f64()?, reader.f64()?];
            if features.insert(feature, weights).is_some() {
                return Err(Error::Chunker(
                    "the chunker model has a feature twice".to_string(),
                ));
            }
        }
        if reader.remaining() != 0 {
            return Err(Error::Chunker(format!(
                "the chunker model has {} bytes after its end",
                reader.remaining()
            )));
        }
        Self::new(schema, columns, template, transitions, features)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::schema::Pattern;

    /// A model with the template of CaboCha and the given weights of
    /// feature strings and transitions.
    pub(crate) fn model(features: &[(&str, [f64; 2])], transitions: [[f64; 3]; 3]) -> ChunkerModel {
        ChunkerModel::new(
            Schema::ipadic(),
            vec![Column::Surface, Column::PartOfSpeech],
            Template::cabocha(),
            transitions,
            features
                .iter()
                .map(|(feature, weights)| (feature.to_string(), *weights))
                .collect(),
        )
        .unwrap()
    }

    /// The error of bytes that are not a model.
    fn error(bytes: &[u8]) -> String {
        ChunkerModel::from_bytes(bytes).unwrap_err().to_string()
    }

    #[test]
    fn a_model_gives_back_what_it_was_made_of() {
        let model = model(
            &[("U02:は", [-1.5, 2.0]), ("U12:名詞", [0.5, 0.0])],
            [[0.0, 0.25, 0.0], [0.0, -0.5, 1.0], [0.75, 0.0, 0.0]],
        );
        assert_eq!(model.schema(), &Schema::ipadic());
        assert_eq!(model.columns(), [Column::Surface, Column::PartOfSpeech]);
        assert_eq!(model.template(), &Template::cabocha());
        assert_eq!(model.feature("U02:は"), Some([-1.5, 2.0]));
        assert_eq!(model.feature("U02:が"), None);
        assert_eq!(model.transition(0, 1), 0.25);
        assert_eq!(model.transition(1, 2), 1.0);
        assert_eq!(model.transition(2, 0), 0.75);
        assert_eq!(model.transition(3, 0), 0.0);
        assert_eq!(model.feature_count(), 2);
        // 3 weights of features and 4 of transitions are not 0.
        assert_eq!(model.weight_count(), 7);
    }

    #[test]
    fn bytes_are_read_back_as_the_same_model() {
        let original = ChunkerModel::new(
            Schema {
                part_of_speech: 1..3,
                conjugation_type: None,
                conjugation_form: Some(7),
                base_form: None,
                // The patterns and the symbols go to the bytes too.
                function_words: vec![Pattern::new(&["助詞"])],
                not_heads: vec![Pattern::new(&["名詞", "非自立"]).after(Pattern::default())],
                symbols: Schema::ipadic().symbols,
            },
            vec![
                Column::Surface,
                Column::PartOfSpeech,
                Column::ConjugationForm,
            ],
            Template::parse("U00:%x[0,0]\nU01:%x[-1,1]/%x[0,2]\nB\n").unwrap(),
            [
                [0.0, 0.25, -3.0],
                [1e-300, -0.5, 1.0],
                [0.75, f64::MAX, -0.0],
            ],
            [
                ("U00:彼".to_string(), [-1.5, 2.0]),
                ("U01:_B-1/連用形".to_string(), [0.1, -0.2]),
                ("U00:".to_string(), [0.0, 3.0]),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
        let bytes = original.to_bytes();
        let read = ChunkerModel::from_bytes(&bytes).unwrap();
        assert_eq!(read, original);
        // The same model gives the same bytes, whatever the order of its
        // table.
        assert_eq!(read.to_bytes(), bytes);
        assert_eq!(&bytes[..8], b"LDCHUNK\0");
    }

    #[test]
    fn refuses_bytes_that_are_not_a_model() {
        let bytes = model(&[("U02:は", [-1.5, 2.0])], [[0.0; 3]; 3]).to_bytes();

        assert_eq!(
            error(b"LDCHUNK"),
            "the chunker model ends early: 8 bytes expected at 0, 7 left"
        );
        let mut other = bytes.clone();
        other[0] = b'X';
        assert_eq!(
            error(&other),
            "not a chunker model: the magic number is missing"
        );
        // Version 1 held the columns of the schema only; it is not read.
        for version in [1, 3] {
            let mut other = bytes.clone();
            other[8] = version;
            assert_eq!(
                error(&other),
                format!("the chunker model has the version {version}, and this crate reads 2")
            );
        }
        // Cut anywhere, the bytes end early.
        for length in [12, 30, bytes.len() - 1] {
            let message = error(&bytes[..length]);
            assert!(
                message.starts_with("the chunker model ends early"),
                "{message}"
            );
        }
        let mut other = bytes.clone();
        other.push(0);
        assert_eq!(error(&other), "the chunker model has 1 bytes after its end");
    }

    #[test]
    fn refuses_bytes_with_content_that_is_no_model() {
        let bytes = model(&[("U02:は", [-1.5, 2.0])], [[0.0; 3]; 3]).to_bytes();
        // The columns start after the magic number, the version and the
        // schema: their count, then one code each.
        let mut schema = Vec::new();
        Schema::ipadic().write(&mut schema);
        let columns = 8 + 4 + schema.len();
        assert_eq!(&bytes[columns..columns + 6], [2, 0, 0, 0, 0, 1]);

        let mut other = bytes.clone();
        other[columns + 5] = 9;
        assert_eq!(error(&other), "the chunker model has the unknown column 9");

        // One column for a template that reads two.
        let mut other = bytes.clone();
        other[columns] = 1;
        other.remove(columns + 5);
        assert_eq!(
            error(&other),
            "the template reads 2 columns, and 1 are given"
        );

        // A weight that is not a number: the last 8 bytes are the weight of
        // `I` of the only feature.
        let mut other = bytes.clone();
        let last = other.len() - 8;
        other[last..].copy_from_slice(&f64::NAN.to_le_bytes());
        assert_eq!(error(&other), "a weight is not finite");
    }

    #[test]
    fn refuses_bytes_with_a_feature_twice() {
        let mut bytes = model(&[("aa", [1.0, 0.0]), ("ab", [0.0, 1.0])], [[0.0; 3]; 3]).to_bytes();
        // The second feature string, after its length: `ab` becomes `aa`.
        let second = bytes
            .windows(6)
            .position(|window| window == [2, 0, 0, 0, b'a', b'b'])
            .unwrap();
        bytes[second + 5] = b'a';
        assert_eq!(error(&bytes), "the chunker model has a feature twice");
    }

    #[test]
    fn new_refuses_a_template_with_more_columns_and_weights_that_are_not_finite() {
        let make = |columns: Vec<Column>, weight: f64| {
            ChunkerModel::new(
                Schema::ipadic(),
                columns,
                Template::cabocha(),
                [[0.0, weight, 0.0], [0.0; 3], [0.0; 3]],
                HashMap::new(),
            )
        };
        assert_eq!(
            make(vec![Column::Surface], 0.0).unwrap_err().to_string(),
            "the template reads 2 columns, and 1 are given"
        );
        assert_eq!(
            make(vec![Column::Surface, Column::PartOfSpeech], f64::INFINITY)
                .unwrap_err()
                .to_string(),
            "a weight is not finite"
        );
        assert!(make(vec![Column::Surface, Column::PartOfSpeech], 1.0).is_ok());
    }
}

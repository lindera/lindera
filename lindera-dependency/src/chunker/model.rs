//! The model of the chunker: its weights, and what makes its features.

use std::collections::HashMap;

use super::columns::Column;
use super::template::Template;
use crate::bytes::{push_string, push_u32};
use crate::error::{Error, Result};
use crate::model::table;
use crate::schema::Schema;

/// The width of the weights of a feature string in a model file: two
/// numbers of 8 bytes, the weight of `B` and the weight of `I`.
pub(crate) const WEIGHTS_WIDTH: usize = 16;

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

/// What a chunker reads of a model: what makes the feature strings, and
/// their weights.
///
/// [`ChunkerModel`] holds them in memory, as training gives them or as they
/// are made by hand; the chunker of a model file
/// ([`Model::chunker`](crate::model::Model::chunker)) reads them on the
/// bytes of the file.
pub trait ChunkerData {
    /// The schema of the dictionary the model is for.
    ///
    /// # Returns
    ///
    /// The schema.
    fn schema(&self) -> &Schema;

    /// The columns the template reads.
    ///
    /// # Returns
    ///
    /// The columns, in the order of their numbers in the template.
    fn columns(&self) -> &[Column];

    /// The template.
    ///
    /// # Returns
    ///
    /// The template.
    fn template(&self) -> &Template;

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
    fn transition(&self, from: usize, to: usize) -> f64;

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
    fn feature(&self, feature: &str) -> Option<[f64; 2]>;
}

/// The model of the chunker, in memory.
///
/// It holds the weights, and with them what makes the feature strings they
/// are for: the schema of the dictionary, the columns and the template.
/// Training and inference then cannot disagree on the features.
///
/// It is what training gives. To keep it, put it into a model file with
/// [`ModelBuilder`](crate::model::ModelBuilder).
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

    /// Appends the model to bytes, as the chunker part of a model file.
    ///
    /// The columns, the template, the transitions, and the table of the
    /// feature strings with their weights. The schema is not written: a
    /// model file holds it once, for all its parts. The same model gives
    /// the same bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    pub(crate) fn write_part(&self, bytes: &mut Vec<u8>) {
        push_u32(bytes, self.columns.len() as u32);
        for column in &self.columns {
            bytes.push(column.code());
        }

        push_string(bytes, &self.template.to_string());

        for weight in self.transitions.iter().flatten() {
            bytes.extend_from_slice(&weight.to_le_bytes());
        }

        let mut features: Vec<(&str, [u8; WEIGHTS_WIDTH])> = self
            .features
            .iter()
            .map(|(feature, weights)| {
                let mut value = [0; WEIGHTS_WIDTH];
                value[..8].copy_from_slice(&weights[0].to_le_bytes());
                value[8..].copy_from_slice(&weights[1].to_le_bytes());
                (feature.as_str(), value)
            })
            .collect();
        features.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
        let entries: Vec<(&str, &[u8])> = features
            .iter()
            .map(|(feature, value)| (*feature, value.as_slice()))
            .collect();
        table::write(bytes, WEIGHTS_WIDTH, &entries);
    }
}

impl ChunkerData for ChunkerModel {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn columns(&self) -> &[Column] {
        &self.columns
    }

    fn template(&self) -> &Template {
        &self.template
    }

    fn transition(&self, from: usize, to: usize) -> f64 {
        ChunkerModel::transition(self, from, to)
    }

    fn feature(&self, feature: &str) -> Option<[f64; 2]> {
        ChunkerModel::feature(self, feature)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

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

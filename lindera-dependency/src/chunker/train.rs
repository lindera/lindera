//! Training the chunker, with the conditional random fields of
//! `lindera-crf`.
//!
//! Each sentence becomes a lattice with, at each morpheme, one edge per
//! label; the first morpheme has the edge of `B` only. A feature of
//! `lindera-crf` stands for a pair of a feature string of the template and a
//! label, and the bigram feature of an edge is its label, so that the
//! bigram weights are the transitions between labels. After training, the
//! weights that are not 0 are copied into a [`ChunkerModel`], which needs
//! `lindera-crf` no more.

use std::collections::HashMap;
use std::num::NonZeroU32;

use lindera_crf::{Edge, FeatureProvider, FeatureSet, Lattice, RawModel, StopReason, Trainer};

use super::columns::Column;
use super::extract;
use super::model::{ChunkerModel, Label};
use super::template::Template;
use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::schema::Schema;
use crate::sentence::Sentence;

/// The regularization of the training.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regularization {
    /// L1: most weights become 0, and the model is small.
    L1,
    /// L2: every feature of the training data keeps a weight.
    L2,
}

/// The settings of a training.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainOptions {
    /// The schema of the dictionary of the training data.
    pub schema: Schema,
    /// The columns the template reads, in the order of their numbers in the
    /// template.
    pub columns: Vec<Column>,
    /// The template.
    pub template: Template,
    /// The regularization.
    pub regularization: Regularization,
    /// The coefficient of the regularization: the larger, the smaller the
    /// weights. With [`Regularization::L2`] it is `1 / C` of CRF++.
    pub lambda: f64,
    /// The largest number of iterations of the optimizer.
    pub max_iterations: u64,
    /// The number of threads.
    pub threads: usize,
}

impl Default for TrainOptions {
    /// The schema of IPADIC; the surface, the part of speech and the
    /// conjugation form as columns, with the default template; L1
    /// regularization with the coefficient 0.5; at most 1,000 iterations;
    /// one thread. The columns, the template and the regularization are the
    /// ones a cross-validation on UD Japanese-GSD chose (#1421).
    fn default() -> Self {
        Self {
            schema: Schema::ipadic(),
            columns: vec![
                Column::Surface,
                Column::PartOfSpeech,
                Column::ConjugationForm,
            ],
            template: Template::default(),
            regularization: Regularization::L1,
            lambda: 0.5,
            max_iterations: 1000,
            threads: 1,
        }
    }
}

/// A trained chunker model, and how its training ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Trained {
    /// The model.
    pub model: ChunkerModel,
    /// The number of iterations the optimizer ran.
    pub iterations: u64,
    /// Whether the optimizer converged; `false` when it stopped at the
    /// largest number of iterations or could make no progress.
    pub converged: bool,
}

/// The feature of `lindera-crf` for a feature string and a label.
///
/// # Arguments
///
/// * `string` - The index of the feature string, from 0.
/// * `label` - The label.
///
/// # Returns
///
/// The feature, from 1: the strings in turn, `B` before `I`.
fn feature_id(string: u32, label: Label) -> Option<NonZeroU32> {
    string
        .checked_mul(2)?
        .checked_add(1 + label.index() as u32)
        .and_then(NonZeroU32::new)
}

/// The bigram feature of an edge with a label: 1 for `B`, 2 for `I`.
///
/// # Arguments
///
/// * `label` - The label.
///
/// # Returns
///
/// The feature.
fn bigram_id(label: Label) -> Option<NonZeroU32> {
    NonZeroU32::new(label.index() as u32 + 1)
}

/// An error of this module from an error of `lindera-crf`.
///
/// # Arguments
///
/// * `error` - The error.
///
/// # Returns
///
/// [`Error::Chunker`] with its message.
fn crf_error(error: lindera_crf::errors::RucrfError) -> Error {
    Error::Chunker(format!("training failed: {error}"))
}

/// Adds the edge of a label at a position of a lattice.
///
/// # Arguments
///
/// * `lattice` - The lattice.
/// * `provider` - Where the features of the edge are registered.
/// * `position` - The position.
/// * `label` - The label.
/// * `strings` - The indices of the feature strings of the position that
///   have features.
///
/// # Returns
///
/// The label `lindera-crf` gave the edge, or [`Error::Chunker`].
fn add_edge(
    lattice: &mut Lattice,
    provider: &mut FeatureProvider,
    position: usize,
    label: Label,
    strings: &[u32],
) -> Result<NonZeroU32> {
    let too_many = || Error::Chunker("too many feature strings".to_string());
    let unigram = strings
        .iter()
        .map(|&string| feature_id(string, label).ok_or_else(too_many))
        .collect::<Result<Vec<NonZeroU32>>>()?;
    let bigram = [bigram_id(label)];
    let edge_label = provider
        .add_feature_set(FeatureSet::new(&unigram, &bigram, &bigram))
        .map_err(crf_error)?;
    lattice
        .add_edge(position, Edge::new(position + 1, edge_label))
        .map_err(crf_error)?;
    Ok(edge_label)
}

/// The model of `lindera-crf` after training, and what its features stand
/// for.
struct Raw {
    /// The model.
    model: RawModel,
    /// The index of each feature string.
    strings: HashMap<String, u32>,
    /// The number of iterations the optimizer ran.
    iterations: u64,
    /// Whether the optimizer converged.
    converged: bool,
}

/// Trains a model of `lindera-crf`.
///
/// # Arguments
///
/// * `sentences` - The training data.
/// * `options` - The settings.
///
/// # Returns
///
/// The model, or [`Error::Chunker`]: see [`train`].
fn train_raw<M: Morpheme>(sentences: &[Sentence<M>], options: &TrainOptions) -> Result<Raw> {
    if options.template.columns() > options.columns.len() {
        return Err(Error::Chunker(format!(
            "the template reads {} columns, and {} are given",
            options.template.columns(),
            options.columns.len()
        )));
    }

    let mut strings: HashMap<String, u32> = HashMap::new();
    let mut provider = FeatureProvider::new();
    let mut lattices = Vec::new();
    for (index, sentence) in sentences.iter().enumerate() {
        sentence.check()?;
        if sentence.morphemes.is_empty() {
            continue;
        }
        if sentence.bunsetsu.is_empty() {
            return Err(Error::Chunker(format!(
                "sentence {} of the training data has no bunsetsu",
                index + 1
            )));
        }
        let mut gold = vec![Label::Inside; sentence.morphemes.len()];
        for bunsetsu in &sentence.bunsetsu {
            gold[bunsetsu.morphemes.start] = Label::Begin;
        }

        let features = extract(
            &options.schema,
            &options.columns,
            &options.template,
            &sentence.morphemes,
        );
        let mut lattice = Lattice::new(features.len()).map_err(crf_error)?;
        for (position, features) in features.into_iter().enumerate() {
            let ids: Vec<u32> = features
                .into_iter()
                .map(|feature| {
                    let next = strings.len() as u32;
                    *strings.entry(feature).or_insert(next)
                })
                .collect();
            // The gold edge first: `lindera-crf` takes the first edge of a
            // position as the positive example. The first morpheme always
            // starts a bunsetsu, so it has no other edge.
            add_edge(&mut lattice, &mut provider, position, gold[position], &ids)?;
            if position > 0 {
                let other = match gold[position] {
                    Label::Begin => Label::Inside,
                    Label::Inside => Label::Begin,
                };
                add_edge(&mut lattice, &mut provider, position, other, &ids)?;
            }
        }
        lattices.push(lattice);
    }
    if lattices.is_empty() {
        return Err(Error::Chunker(
            "the training data has no sentence".to_string(),
        ));
    }

    let regularization = match options.regularization {
        Regularization::L1 => lindera_crf::Regularization::L1,
        Regularization::L2 => lindera_crf::Regularization::L2,
    };
    let trainer = Trainer::new()
        .regularization(regularization, options.lambda)
        .map_err(crf_error)?
        .max_iter(options.max_iterations)
        .map_err(crf_error)?
        .n_threads(options.threads)
        .map_err(crf_error)?;
    let (model, stats) = trainer.train_with_stats(&lattices, provider);
    Ok(Raw {
        model,
        strings,
        iterations: stats.iterations,
        converged: stats.stop_reason == StopReason::Converged,
    })
}

/// Copies the weights of a model of `lindera-crf` into a model of the
/// chunker.
///
/// # Arguments
///
/// * `raw` - The trained model.
/// * `options` - The settings it was trained with.
///
/// # Returns
///
/// The model, with the feature strings that have a weight that is not 0;
/// or the error of [`ChunkerModel::new`].
fn chunker_model(raw: &Raw, options: &TrainOptions) -> Result<ChunkerModel> {
    let mut features = HashMap::new();
    for (feature, &string) in &raw.strings {
        let mut weights = [0.0; 2];
        for label in Label::ALL {
            if let Some(id) = feature_id(string, label) {
                weights[label.index()] = raw.model.unigram_weight(&[id]);
            }
        }
        if weights != [0.0; 2] {
            features.insert(feature.clone(), weights);
        }
    }

    // The bigram weight of `lindera-crf` between a left and a right bigram
    // feature; 0 stands for the start of the sentence on the left and for
    // its end on the right. The bigram features are the label numbers.
    let mut transitions = [[0.0; 3]; 3];
    let indices = raw.model.bigram_weight_indices();
    for (from, row) in transitions.iter_mut().enumerate() {
        for (to, weight) in row.iter_mut().enumerate() {
            if let Some(&index) = indices.get(from).and_then(|map| map.get(&(to as u32))) {
                *weight = raw.model.weights()[index as usize];
            }
        }
    }

    ChunkerModel::new(
        options.schema.clone(),
        options.columns.clone(),
        options.template.clone(),
        transitions,
        features,
    )
}

/// Trains a chunker model.
///
/// # Arguments
///
/// * `sentences` - The training data: sentences with their bunsetsu. A
///   sentence without morphemes is skipped.
/// * `options` - The settings.
///
/// # Returns
///
/// The model and how the training ended, or an error:
/// [`Error::Structure`] for a sentence [`Sentence::check`] refuses,
/// [`Error::Chunker`] for a template that reads more columns than are
/// given, a sentence without bunsetsu, training data without a sentence, or
/// settings `lindera-crf` refuses (a coefficient that is negative or not
/// finite, no iteration, no thread).
pub fn train<M: Morpheme>(sentences: &[Sentence<M>], options: &TrainOptions) -> Result<Trained> {
    let raw = train_raw(sentences, options)?;
    Ok(Trained {
        model: chunker_model(&raw, options)?,
        iterations: raw.iterations,
        converged: raw.converged,
    })
}

#[cfg(test)]
mod tests {
    use lindera_crf::Model;

    use super::*;
    use crate::chunker::Chunker;
    use crate::lattice::{self, LatticeMorpheme};
    use crate::model::{Model as ModelFile, ModelBuilder};

    /// The labels `search_best_path` of `lindera-crf` gives a sentence, for
    /// the trained model.
    fn crf_labels(raw: &mut Raw, features: &[Vec<String>]) -> Vec<Label> {
        let mut lattice = Lattice::new(features.len()).unwrap();
        let mut labels = HashMap::new();
        for (position, features) in features.iter().enumerate() {
            let ids: Vec<u32> = features
                .iter()
                .filter_map(|feature| raw.strings.get(feature).copied())
                .collect();
            for label in Label::ALL {
                if position == 0 && label == Label::Inside {
                    continue;
                }
                let edge_label = add_edge(
                    &mut lattice,
                    raw.model.feature_provider(),
                    position,
                    label,
                    &ids,
                )
                .unwrap();
                labels.insert(edge_label, label);
            }
        }
        let (path, _) = raw.model.search_best_path(&lattice);
        path.iter().map(|edge| labels[&edge.label()]).collect()
    }

    /// The labels of the bunsetsu of a sentence.
    fn gold_labels<M>(sentence: &Sentence<M>) -> Vec<Label> {
        let mut labels = vec![Label::Inside; sentence.morphemes.len()];
        for bunsetsu in &sentence.bunsetsu {
            labels[bunsetsu.morphemes.start] = Label::Begin;
        }
        labels
    }

    /// Checks that the chunker and `search_best_path` give every sentence
    /// the same labels, and returns the number of sentences whose labels
    /// are those of the answer.
    fn same_labels(
        raw: &mut Raw,
        options: &TrainOptions,
        sentences: &[Sentence<LatticeMorpheme>],
    ) -> usize {
        let chunker = Chunker::new(chunker_model(raw, options).unwrap());
        let mut right = 0;
        for (index, sentence) in sentences.iter().enumerate() {
            if sentence.morphemes.is_empty() {
                continue;
            }
            let features = chunker.features(&sentence.morphemes);
            let labels = chunker.labels(&features);
            assert_eq!(labels, crf_labels(raw, &features), "sentence {}", index + 1);
            if labels == gold_labels(sentence) {
                right += 1;
            }
        }
        right
    }

    /// A sentence in the lattice format, of bunsetsu given as their
    /// morphemes, each `surface/part of speech`.
    fn sentence(bunsetsu: &[&[&str]]) -> String {
        let mut text = String::new();
        for (index, morphemes) in bunsetsu.iter().enumerate() {
            let head = if index + 1 == bunsetsu.len() {
                -1
            } else {
                index as i32 + 1
            };
            text.push_str(&format!("* {index} {head}D \n"));
            for morpheme in *morphemes {
                let (surface, pos) = morpheme.split_once('/').unwrap();
                text.push_str(&format!("{surface}\t{pos},*,*,*,*,*,{surface},*,*\n"));
            }
        }
        text.push_str("EOS\n");
        text
    }

    /// Training data: nouns with particles and a verb, where a particle
    /// continues a bunsetsu; compound nouns, where a noun continues one
    /// after a noun and starts one after a particle, which only the features
    /// can tell; and runs of the same morpheme `x`, cut after every second
    /// one, which only the transitions between labels can tell.
    fn training_data() -> Vec<Sentence<LatticeMorpheme>> {
        let text = [
            sentence(&[
                &["彼/名詞", "は/助詞"],
                &["本/名詞", "を/助詞"],
                &["読む/動詞"],
            ]),
            sentence(&[
                &["私/名詞", "が/助詞"],
                &["水/名詞", "を/助詞"],
                &["飲む/動詞"],
            ]),
            sentence(&[&["犬/名詞", "が/助詞"], &["走る/動詞"]]),
            sentence(&[
                &["猫/名詞", "は/助詞"],
                &["魚/名詞", "を/助詞"],
                &["食べる/動詞"],
            ]),
            sentence(&[&["本/名詞", "を/助詞"], &["読む/動詞"]]),
            sentence(&[&["東京/名詞", "都/名詞", "に/助詞"], &["行く/動詞"]]),
            sentence(&[
                &["彼/名詞", "は/助詞"],
                &["日本/名詞", "語/名詞", "を/助詞"],
                &["話す/動詞"],
            ]),
            sentence(&[
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
            ]),
            sentence(&[
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
            ]),
        ]
        .concat();
        lattice::parse(&text).unwrap()
    }

    /// Sentences the training data does not hold, some with words it does
    /// not have.
    fn other_data() -> Vec<Sentence<LatticeMorpheme>> {
        let text = [
            sentence(&[
                &["犬/名詞", "は/助詞"],
                &["水/名詞", "を/助詞"],
                &["飲む/動詞"],
            ]),
            sentence(&[&["鳥/名詞", "が/助詞"], &["飛ぶ/動詞"]]),
            sentence(&[&["読む/動詞"]]),
            sentence(&[&["は/助詞"], &["を/助詞", "が/助詞"]]),
            sentence(&[
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
                &["x/記号", "x/記号"],
            ]),
            sentence(&[&["未知/感動詞", "語/感動詞"], &["x/記号"]]),
        ]
        .concat();
        lattice::parse(&text).unwrap()
    }

    /// Settings for the small data of these tests.
    fn options(regularization: Regularization, lambda: f64) -> TrainOptions {
        TrainOptions {
            regularization,
            lambda,
            ..TrainOptions::default()
        }
    }

    #[test]
    fn the_chunker_gives_the_labels_of_search_best_path() {
        for (regularization, lambda) in [
            (Regularization::L2, 0.01),
            (Regularization::L2, 1.0),
            (Regularization::L1, 0.01),
            (Regularization::L1, 0.5),
        ] {
            let options = options(regularization, lambda);
            let training = training_data();
            let mut raw = train_raw(&training, &options).unwrap();
            // The training data is reproduced with a small coefficient.
            let right = same_labels(&mut raw, &options, &training);
            if lambda == 0.01 {
                assert_eq!(right, training.len(), "{regularization:?} {lambda}");
            }
            same_labels(&mut raw, &options, &other_data());
        }
    }

    #[test]
    fn the_transitions_between_labels_are_learned() {
        let options = options(Regularization::L2, 0.01);
        let trained = train(&training_data(), &options).unwrap();
        let chunker = Chunker::new(trained.model);
        // A run of `x` longer than any of the training data: the middle
        // positions have the same features, and alternate.
        let run: Vec<LatticeMorpheme> = (0..10)
            .map(|_| LatticeMorpheme::new("x", "記号,*,*,*,*,*,x,*,*"))
            .collect();
        assert_eq!(
            chunker.chunk(&run).unwrap(),
            [0..2, 2..4, 4..6, 6..8, 8..10]
        );
        // After B, I is preferred to B, and after I, B to I.
        let model = chunker.model();
        assert!(model.transition(1, 2) > model.transition(1, 1));
        assert!(model.transition(2, 1) > model.transition(2, 2));
        // The direction of a transition: no sentence starts with I, as the
        // first position has no such edge, and the runs of `x` end with I.
        assert_eq!(model.transition(0, 2), 0.0);
        assert_ne!(model.transition(2, 0), 0.0);
    }

    #[test]
    fn a_trained_model_chunks_and_goes_through_bytes() {
        let options = options(Regularization::L2, 0.01);
        let trained = train(&training_data(), &options).unwrap();
        assert!(trained.iterations > 0);
        let model = trained.model;
        assert_eq!(model.schema(), &options.schema);
        assert_eq!(model.columns(), options.columns);
        assert_eq!(model.template(), &options.template);

        // Through a model file, the chunker reads the same weights where
        // they are in the bytes.
        let bytes = ModelBuilder::new("test", options.schema.clone())
            .unwrap()
            .chunker(&model)
            .unwrap()
            .to_bytes();
        let read = ModelFile::new(bytes).unwrap();
        let chunker = read.chunker().unwrap();
        assert_eq!(chunker.model().feature_count(), model.feature_count());
        assert_eq!(chunker.model().weight_count(), model.weight_count());
        for sentence in training_data() {
            let expected: Vec<_> = sentence
                .bunsetsu
                .iter()
                .map(|bunsetsu| bunsetsu.morphemes.clone())
                .collect();
            assert_eq!(chunker.chunk(&sentence.morphemes).unwrap(), expected);
        }
    }

    #[test]
    fn l1_leaves_fewer_weights_than_l2() {
        let training = training_data();
        let l2 = train(&training, &options(Regularization::L2, 0.1))
            .unwrap()
            .model;
        let l1 = train(&training, &options(Regularization::L1, 0.1))
            .unwrap()
            .model;
        assert!(
            l1.feature_count() < l2.feature_count(),
            "{} and {}",
            l1.feature_count(),
            l2.feature_count()
        );
        assert!(l1.weight_count() < l2.weight_count());
        // No feature string of a model has only weights that are 0.
        assert!(l1.weight_count() >= l1.feature_count());
    }

    #[test]
    fn refuses_training_data_and_settings_it_cannot_use() {
        let error = |sentences: &[Sentence<LatticeMorpheme>], options: &TrainOptions| {
            train(sentences, options).unwrap_err().to_string()
        };
        let training = training_data();
        let default = TrainOptions::default();

        assert_eq!(error(&[], &default), "the training data has no sentence");
        // Morphemes without bunsetsu.
        let morphemes_only = lattice::parse("彼\t名詞\nEOS\n").unwrap();
        assert_eq!(
            error(&morphemes_only, &default),
            "sentence 1 of the training data has no bunsetsu"
        );
        // A template over three columns, and one column.
        let one_column = TrainOptions {
            columns: vec![Column::Surface],
            ..TrainOptions::default()
        };
        assert_eq!(
            error(&training, &one_column),
            "the template reads 3 columns, and 1 are given"
        );
        // Settings `lindera-crf` refuses.
        for options in [
            TrainOptions {
                lambda: -1.0,
                ..TrainOptions::default()
            },
            TrainOptions {
                max_iterations: 0,
                ..TrainOptions::default()
            },
            TrainOptions {
                threads: 0,
                ..TrainOptions::default()
            },
        ] {
            let message = error(&training, &options);
            assert!(message.starts_with("training failed: "), "{message}");
        }
    }

    /// The same check as
    /// `the_chunker_gives_the_labels_of_search_best_path`, on real data:
    /// run with `--ignored` and, in `LINDERA_DEPENDENCY_CHUNKER_TRAIN` and
    /// `LINDERA_DEPENDENCY_CHUNKER_EVAL`, the paths of a training file and
    /// of files to label (separated by `:`), in the lattice format.
    #[test]
    #[ignore = "needs training data: see the comment"]
    fn the_chunker_gives_the_labels_of_search_best_path_on_real_data() {
        let read = |path: &str| {
            let text = std::fs::read_to_string(path).unwrap();
            lattice::parse(&text).unwrap()
        };
        let training = read(&std::env::var("LINDERA_DEPENDENCY_CHUNKER_TRAIN").unwrap());
        let files = std::env::var("LINDERA_DEPENDENCY_CHUNKER_EVAL").unwrap();
        let threads = std::thread::available_parallelism().map_or(1, usize::from);
        for (regularization, lambda) in [(Regularization::L1, 1.0), (Regularization::L2, 1.0)] {
            let options = TrainOptions {
                regularization,
                lambda,
                threads,
                ..TrainOptions::default()
            };
            let mut raw = train_raw(&training, &options).unwrap();
            for path in files.split(':') {
                let sentences = read(path);
                let right = same_labels(&mut raw, &options, &sentences);
                println!(
                    "{regularization:?} {lambda} {path}: {} sentences with the same labels, {right} as the answer",
                    sentences.len()
                );
            }
        }
    }
}

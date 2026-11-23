//! Chunking: segmenting the morphemes of a sentence into bunsetsu.
//!
//! The chunker decides, for every morpheme, whether it starts a bunsetsu
//! (`B`) or continues one (`I`), with a conditional random field over the
//! two labels, as CaboCha does.
//!
//! It is in three parts, so that each can be replaced:
//!
//! - **The features of a position** are strings. The [`Column`]s of each
//!   morpheme (its surface, its part of speech, ...) are taken through the
//!   [`Schema`] of the dictionary, and a [`Template`] in the
//!   format of CRF++ expands them ([`Chunker::features`]).
//! - **The model** ([`ChunkerModel`]) is a table from feature string to the
//!   weights of `B` and `I`, and the weights of the transitions. It holds
//!   the schema, the columns and the template it was trained with.
//! - **Decoding** ([`Chunker::labels`]) finds the best labels for feature
//!   strings, by Viterbi. It does not know where the strings come from.
//!
//! Inference needs nothing but this crate. Training, behind the `train`
//! feature, uses `lindera-crf`.
//!
//! # Examples
//!
//! A model made by hand, with one weight: a morpheme after a particle
//! starts a bunsetsu.
//!
//! ```
//! use std::collections::HashMap;
//!
//! use lindera_dependency::chunker::{Chunker, ChunkerModel, Column, Template};
//! use lindera_dependency::lattice::LatticeMorpheme;
//! use lindera_dependency::Schema;
//!
//! // U11 is the part of speech of the morpheme before: `B` gets 1.0.
//! let mut features = HashMap::new();
//! features.insert("U11:助詞-係助詞".to_string(), [1.0, 0.0]);
//! features.insert("U11:助詞-格助詞-一般".to_string(), [1.0, 0.0]);
//! // Without a feature for `B`, a morpheme continues the bunsetsu: `I`
//! // after anything gets 0.5.
//! let transitions = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.5], [0.0, 0.0, 0.5]];
//! let model = ChunkerModel::new(
//!     Schema::ipadic(),
//!     vec![Column::Surface, Column::PartOfSpeech],
//!     Template::cabocha(),
//!     transitions,
//!     features,
//! )?;
//! let chunker = Chunker::new(model);
//!
//! let morphemes = [
//!     LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
//!     LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
//!     LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
//!     LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
//!     LatticeMorpheme::new("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
//! ];
//! assert_eq!(chunker.chunk(&morphemes), [0..2, 2..4, 4..5]);
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

pub mod columns;
mod decode;
pub mod model;
pub mod template;
#[cfg(feature = "train")]
pub mod train;

use std::ops::Range;

pub use columns::{Column, parse_columns};
pub use model::{ChunkerModel, Label};
pub use template::Template;
#[cfg(feature = "train")]
pub use train::{Regularization, TrainOptions, Trained, train};

use crate::morpheme::Morpheme;
use crate::schema::Schema;

/// The feature strings of each morpheme of a sentence.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morphemes.
/// * `columns` - The columns the template reads.
/// * `template` - The template.
/// * `morphemes` - The morphemes of the sentence.
///
/// # Returns
///
/// For each morpheme, one string per unigram line of the template.
pub(crate) fn extract<M: Morpheme>(
    schema: &Schema,
    columns: &[Column],
    template: &Template,
    morphemes: &[M],
) -> Vec<Vec<String>> {
    let rows = columns::rows(schema, columns, morphemes);
    (0..rows.len())
        .map(|position| template.expand(&rows, position))
        .collect()
}

/// Segments the morphemes of a sentence into bunsetsu.
///
/// A chunker is made from a model once and used from any number of threads:
/// chunking takes `&self` and keeps no state.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunker {
    /// The model.
    model: ChunkerModel,
}

impl Chunker {
    /// Makes a chunker.
    ///
    /// # Arguments
    ///
    /// * `model` - The model.
    ///
    /// # Returns
    ///
    /// The chunker.
    pub fn new(model: ChunkerModel) -> Self {
        Self { model }
    }

    /// The model of the chunker.
    ///
    /// # Returns
    ///
    /// The model.
    pub fn model(&self) -> &ChunkerModel {
        &self.model
    }

    /// The feature strings of each morpheme of a sentence, made with the
    /// schema, the columns and the template of the model.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the sentence.
    ///
    /// # Returns
    ///
    /// For each morpheme, one string per unigram line of the template.
    pub fn features<M: Morpheme>(&self, morphemes: &[M]) -> Vec<Vec<String>> {
        extract(
            self.model.schema(),
            self.model.columns(),
            self.model.template(),
            morphemes,
        )
    }

    /// The best labels for the feature strings of a sentence.
    ///
    /// This is the decoder alone: the strings can come from
    /// [`Chunker::features`] or from another extractor that makes the
    /// strings the model was trained on.
    ///
    /// # Arguments
    ///
    /// * `features` - The feature strings of each position.
    ///
    /// # Returns
    ///
    /// One label per position, the first always [`Label::Begin`]; none for
    /// an empty sentence.
    pub fn labels(&self, features: &[Vec<String>]) -> Vec<Label> {
        decode::decode(&self.model, features)
    }

    /// Segments the morphemes of a sentence into bunsetsu.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the sentence.
    ///
    /// # Returns
    ///
    /// The bunsetsu from left to right, each as a range of indices into
    /// `morphemes`. They cover the morphemes without a gap: the first
    /// morpheme always starts a bunsetsu. An empty sentence has none.
    pub fn chunk<M: Morpheme>(&self, morphemes: &[M]) -> Vec<Range<usize>> {
        let labels = self.labels(&self.features(morphemes));
        let mut bunsetsu: Vec<Range<usize>> = Vec::new();
        for (index, label) in labels.iter().enumerate() {
            match (label, bunsetsu.last_mut()) {
                (Label::Inside, Some(last)) => last.end = index + 1,
                _ => bunsetsu.push(index..index + 1),
            }
        }
        bunsetsu
    }
}

#[cfg(test)]
// A sentence that is one bunsetsu is compared with `[0..n]`: an array of one
// range, which is what is meant.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;
    use crate::chunker::model::tests::model;
    use crate::lattice::LatticeMorpheme;

    /// 彼 は 本 を 読む, with the features of IPADIC.
    fn morphemes() -> Vec<LatticeMorpheme> {
        vec![
            LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
            LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
            LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
            LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
            LatticeMorpheme::new("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
        ]
    }

    /// A chunker for which a particle continues a bunsetsu and anything
    /// else starts one.
    fn chunker() -> Chunker {
        Chunker::new(model(
            &[
                ("U12:助詞-係助詞", [0.0, 1.0]),
                ("U12:助詞-格助詞-一般", [0.0, 1.0]),
            ],
            [[0.0; 3]; 3],
        ))
    }

    #[test]
    fn chunks_a_sentence() {
        let chunker = chunker();
        let morphemes = morphemes();
        assert_eq!(chunker.chunk(&morphemes), [0..2, 2..4, 4..5]);
        assert_eq!(
            chunker.labels(&chunker.features(&morphemes)),
            [
                Label::Begin,
                Label::Inside,
                Label::Begin,
                Label::Inside,
                Label::Begin
            ]
        );
    }

    #[test]
    fn the_features_are_those_of_the_template_on_the_columns() {
        let chunker = chunker();
        let features = chunker.features(&morphemes());
        assert_eq!(features.len(), 5);
        assert_eq!(features[1].len(), 19);
        assert_eq!(features[1][2], "U02:は");
        assert_eq!(features[1][9], "U12:助詞-係助詞");
        assert_eq!(
            features[1][17],
            "U21:名詞-代名詞-一般/助詞-係助詞/名詞-一般"
        );
        assert_eq!(features[4][11], "U14:_B+2");
    }

    #[test]
    fn an_empty_sentence_and_a_sentence_of_one_morpheme() {
        let chunker = chunker();
        let none: [LatticeMorpheme; 0] = [];
        assert!(chunker.chunk(&none).is_empty());
        assert_eq!(chunker.chunk(&morphemes()[..1]), [0..1]);
        // A particle alone is a bunsetsu: the first morpheme starts one.
        assert_eq!(chunker.chunk(&morphemes()[1..2]), [0..1]);
        assert_eq!(chunker.chunk(&morphemes()[1..4]), [0..1, 1..3]);
    }

    #[test]
    fn the_bunsetsu_cover_the_morphemes() {
        // Everything continues: one bunsetsu.
        let chunker = Chunker::new(model(&[], [[0.0; 3], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]));
        assert_eq!(chunker.chunk(&morphemes()), [0..5]);
        // Everything starts: one bunsetsu per morpheme.
        let chunker = Chunker::new(model(&[], [[0.0; 3]; 3]));
        assert_eq!(chunker.chunk(&morphemes()), [0..1, 1..2, 2..3, 3..4, 4..5]);
    }

    #[test]
    fn a_chunker_is_shared_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Chunker>();
        assert_send_sync::<ChunkerModel>();

        let chunker = chunker();
        let morphemes = morphemes();
        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| assert_eq!(chunker.chunk(&morphemes), [0..2, 2..4, 4..5]));
            }
        });
        assert_eq!(chunker.model().feature_count(), 2);
    }
}

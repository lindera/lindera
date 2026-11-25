//! The token filter that analyzes the dependencies of Japanese text.
//!
//! The filter groups the tokens of a text into bunsetsu (文節), finds the
//! bunsetsu each of them depends on, and writes the result on every token
//! ([`Token::dependency`]). It changes nothing else: no token is added,
//! removed or rewritten.
//!
//! It is built with the feature `dependency`, and reads a model file of
//! `lindera-dependency`, which is for one dictionary.
//!
//! # Sentences
//!
//! A text is analyzed sentence by sentence. A sentence ends after a token
//! of `。`, `！` and `？` alone (`!` and `?` as well), when it is not inside
//! brackets; such tokens and closing brackets that follow go to the
//! sentence they end. The full-width period `．` ends a sentence too,
//! unless a digit follows it (`３．１４`). The half-width `.` ends none:
//! it stands in numbers (`4.20`), in `...` and in names. What is left
//! after the last end is a sentence too. The bunsetsu of each sentence are
//! numbered from 0, and [`TokenDependency::sentence`] tells the sentences
//! apart. A line break is not a token, and ends no sentence.
//!
//! # Where the filter goes
//!
//! The model reads the tokens as the dictionary gave them, so the filter
//! comes before any token filter that changes them. Tokens removed or
//! joined by an earlier filter are refused with an error, as far as the
//! positions of the tokens show it; a surface or a feature that an earlier
//! filter rewrote is not seen, and gives a wrong analysis without an error.
//!
//! # Examples
//!
//! ```no_run
//! use std::path::Path;
//!
//! use lindera_analysis::token_filter::BoxTokenFilter;
//! use lindera_analysis::token_filter::japanese_dependency::JapaneseDependencyTokenFilter;
//! use lindera_analysis::tokenizer::Tokenizer;
//! use lindera_segmenter::dictionary::load_dictionary;
//! use lindera_segmenter::mode::Mode;
//! use lindera_segmenter::segmenter::Segmenter;
//!
//! let dictionary = load_dictionary("/path/to/ipadic")?;
//! let mut tokenizer = Tokenizer::new(Segmenter::new(Mode::Normal, dictionary, None));
//! let filter = JapaneseDependencyTokenFilter::from_path(Path::new("ipadic.model"))?;
//! tokenizer.append_token_filter(BoxTokenFilter::from(filter));
//!
//! for token in tokenizer.tokenize("彼は赤い花を見た。")? {
//!     if let Some(dependency) = token.dependency() {
//!         // 彼 is in bunsetsu 0, which depends on bunsetsu 3 (見た。).
//!         println!(
//!             "{}\t{}\t{:?}\t{}",
//!             token.surface,
//!             dependency.bunsetsu,
//!             dependency.head,
//!             dependency.role.as_str()
//!         );
//!     }
//! }
//! # Ok::<(), lindera_segmenter::error::LinderaError>(())
//! ```

use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use lindera_dependency::model::Model;
use lindera_dependency::{Bunsetsu, Morpheme, Schema};
use serde_json::Value;

use crate::token_filter::TokenFilter;
use crate::{LinderaErrorKind, LinderaResult};
use lindera_segmenter::error::LinderaError;
use lindera_segmenter::token::{BunsetsuRole, Token, TokenDependency};

pub const JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME: &str = "japanese_dependency";

pub type JapaneseDependencyTokenFilterConfig = Value;

/// Write on each token its bunsetsu and the bunsetsu that one depends on,
/// with a model of `lindera-dependency`.
///
/// See the [module](self) for how a text is cut into sentences and where
/// the filter goes among the others. The model is shared by the clones of
/// the filter.
#[derive(Clone, Debug)]
pub struct JapaneseDependencyTokenFilter {
    /// The model: its chunker and its parser read their weights in it.
    model: Arc<Model<Vec<u8>>>,
}

impl JapaneseDependencyTokenFilter {
    /// Makes the filter from the bytes of a model file.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes of a model file of `lindera-dependency`.
    ///
    /// # Returns
    ///
    /// The filter, or an error for bytes that are not a model file, or a
    /// model without a chunker or without a parser.
    pub fn from_bytes(bytes: Vec<u8>) -> LinderaResult<Self> {
        let model = Model::new(bytes).map_err(to_error)?;
        if model.chunker().is_none() {
            return Err(LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("the dependency model has no chunker")));
        }
        if model.parser().is_none() {
            return Err(LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("the dependency model has no parser")));
        }
        Ok(Self {
            model: Arc::new(model),
        })
    }

    /// Makes the filter from a model file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of a model file of `lindera-dependency`.
    ///
    /// # Returns
    ///
    /// The filter, or an error when the file cannot be read or is not a
    /// model with a chunker and a parser.
    pub fn from_path(path: &Path) -> LinderaResult<Self> {
        let bytes = fs::read(path).map_err(|error| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "cannot read the dependency model {}: {error}",
                path.display()
            ))
        })?;
        Self::from_bytes(bytes)
            .map_err(|error| error.add_context(format!("the dependency model {}", path.display())))
    }

    /// Makes the filter from its configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - An object with `model`, the path of a model file.
    ///
    /// # Returns
    ///
    /// The filter, or an error when `model` is missing or is not a string,
    /// or as [`JapaneseDependencyTokenFilter::from_path`].
    pub fn from_config(config: &JapaneseDependencyTokenFilterConfig) -> LinderaResult<Self> {
        let path = config.get("model").and_then(Value::as_str).ok_or_else(|| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "{JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME}: `model`, the path of a model file, is required"
            ))
        })?;
        Self::from_path(Path::new(path))
    }

    /// The name of the dictionary the model of the filter is for.
    ///
    /// # Returns
    ///
    /// The name, as `ipadic`.
    pub fn dictionary(&self) -> &str {
        self.model.dictionary()
    }
}

/// Turns an error of `lindera-dependency` into one of Lindera.
fn to_error(error: lindera_dependency::Error) -> LinderaError {
    let kind = match error {
        lindera_dependency::Error::Dictionary(_) => LinderaErrorKind::Dictionary,
        lindera_dependency::Error::Io(_) => LinderaErrorKind::Io,
        _ => LinderaErrorKind::Content,
    };
    kind.with_error(error)
}

/// A token as the analysis reads it: its surface and its details, where
/// they are.
struct TokenMorpheme<'t, 'a> {
    /// The surface of the token.
    surface: &'t str,
    /// The details of the token.
    details: &'t [Cow<'a, str>],
}

impl Morpheme for TokenMorpheme<'_, '_> {
    fn surface(&self) -> &str {
        self.surface
    }

    fn feature(&self, index: usize) -> Option<&str> {
        self.details.get(index).map(|detail| detail.as_ref())
    }
}

/// Whether a morpheme ends a sentence.
///
/// # Arguments
///
/// * `morpheme` - The morpheme.
/// * `next` - The morpheme after it, when there is one.
///
/// # Returns
///
/// Whether its surface is of `。`, `！`, `？`, `!` and `?` alone, or is
/// `．` and no digit follows. The half-width `.` is not an end: it is in
/// `4.20` and in `...`.
fn is_sentence_end<M: Morpheme>(morpheme: &M, next: Option<&M>) -> bool {
    let surface = morpheme.surface();
    if surface == "．" {
        return !next.is_some_and(|next| {
            next.surface().starts_with(|letter: char| {
                letter.is_ascii_digit() || ('０'..='９').contains(&letter)
            })
        });
    }
    !surface.is_empty()
        && surface
            .chars()
            .all(|letter| matches!(letter, '。' | '！' | '？' | '!' | '?'))
}

/// Cuts the morphemes of a text into sentences.
///
/// # Arguments
///
/// * `schema` - The schema of the dictionary of the morphemes.
/// * `morphemes` - The morphemes of the text.
///
/// # Returns
///
/// The end of each sentence, as the index after its last morpheme; the last
/// one is the number of morphemes. Nothing for no morpheme.
fn sentence_ends<M: Morpheme>(schema: &Schema, morphemes: &[M]) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut depth = 0usize;
    let mut index = 0;
    while index < morphemes.len() {
        let morpheme = &morphemes[index];
        index += 1;
        if schema.is_open_bracket(morpheme) {
            depth += 1;
        } else if schema.is_close_bracket(morpheme) {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && is_sentence_end(morpheme, morphemes.get(index)) {
            // What follows and still belongs to the sentence.
            while morphemes.get(index).is_some_and(|next| {
                is_sentence_end(next, morphemes.get(index + 1)) || schema.is_close_bracket(next)
            }) {
                index += 1;
            }
            ends.push(index);
        }
    }
    if ends.last().copied().unwrap_or(0) < morphemes.len() {
        ends.push(morphemes.len());
    }
    ends
}

/// The dependency of each morpheme of a sentence.
///
/// # Arguments
///
/// * `sentence` - The number of the sentence in the text.
/// * `bunsetsu` - The bunsetsu of the sentence, with their heads.
/// * `dependencies` - Where the dependencies go, one for each morpheme, in
///   the order of the morphemes.
fn push_dependencies(
    sentence: usize,
    bunsetsu: &[Bunsetsu],
    dependencies: &mut Vec<TokenDependency>,
) {
    for (number, one) in bunsetsu.iter().enumerate() {
        let words = one
            .positions
            .map(|(head, function)| (one.morphemes.start + head, one.morphemes.start + function));
        for index in one.morphemes.clone() {
            let role = match words {
                Some((head, function)) if head == index && function == index => {
                    BunsetsuRole::HeadAndFunction
                }
                Some((head, _)) if head == index => BunsetsuRole::Head,
                Some((_, function)) if function == index => BunsetsuRole::Function,
                _ => BunsetsuRole::Other,
            };
            dependencies.push(TokenDependency {
                sentence: sentence as u32,
                bunsetsu: number as u32,
                head: one.head.map(|head| head as u32),
                role,
                score: one.score.unwrap_or(0.0),
            });
        }
    }
}

impl TokenFilter for JapaneseDependencyTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME
    }

    /// Writes the dependency of every token.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The tokens of a text, as the tokenizer gave them.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when the tokens are of another dictionary than
    /// the one of the model, when an earlier filter removed or joined
    /// tokens, or when a token has fewer details than the model reads. No
    /// token is changed when there is an error.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        let Some(first) = tokens.first() else {
            return Ok(());
        };
        self.model
            .check_dictionary(&first.dictionary.metadata.name)
            .map_err(to_error)?;

        for (index, token) in tokens.iter_mut().enumerate() {
            // The tokenizer numbers its tokens from 0 without a gap; a filter
            // that removes tokens leaves a gap, and one that joins them a
            // token of more than one position.
            if token.position != index || token.position_length != 1 {
                return Err(LinderaErrorKind::Args.with_error(anyhow::anyhow!(
                    "{JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME}: token {index} ({:?}) is at position {} \
                     with a length of {}: an earlier token filter removed or joined tokens; \
                     put this filter before it",
                    token.surface,
                    token.position,
                    token.position_length
                )));
            }
            // Loads the details into the token, where the morphemes below
            // read them.
            token.get_detail(0);
        }

        let mut dependencies = Vec::with_capacity(tokens.len());
        {
            let morphemes: Vec<TokenMorpheme<'_, '_>> = tokens
                .iter()
                .map(|token| TokenMorpheme {
                    surface: token.surface.as_ref(),
                    details: token.details.as_deref().unwrap_or(&[]),
                })
                .collect();
            let missing = || {
                LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!("the dependency model has lost a part"))
            };
            let chunker = self.model.chunker().ok_or_else(missing)?;
            let parser = self.model.parser().ok_or_else(missing)?;
            let schema = self.model.schema();

            let mut start = 0;
            for (sentence, end) in sentence_ends(schema, &morphemes).into_iter().enumerate() {
                let part = &morphemes[start..end];
                let chunks = chunker.chunk(part).map_err(to_error)?;
                let bunsetsu = parser.parse(part, &chunks).map_err(to_error)?;
                push_dependencies(sentence, &bunsetsu, &mut dependencies);
                start = end;
            }
        }
        if dependencies.len() != tokens.len() {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME}: {} dependencies for {} tokens",
                dependencies.len(),
                tokens.len()
            )));
        }

        for (token, dependency) in tokens.iter_mut().zip(dependencies) {
            token.set_dependency(Some(dependency));
        }
        Ok(())
    }
}

#[cfg(test)]
mod sentence_tests {
    use lindera_dependency::Schema;
    use lindera_dependency::lattice::LatticeMorpheme;

    use super::sentence_ends;

    /// The morphemes of a text, from their surfaces: brackets and periods
    /// with the features of IPADIC, anything else as a noun.
    fn morphemes(surfaces: &[&str]) -> Vec<LatticeMorpheme> {
        surfaces
            .iter()
            .map(|surface| {
                let features = match *surface {
                    "。" | "．" | "." => {
                        format!("記号,句点,*,*,*,*,{surface},{surface},{surface}")
                    }
                    "「" | "（" => format!("記号,括弧開,*,*,*,*,{surface},{surface},{surface}"),
                    "」" | "）" => format!("記号,括弧閉,*,*,*,*,{surface},{surface},{surface}"),
                    "！" | "？" | "!" | "?" | "！？" => {
                        format!("記号,一般,*,*,*,*,{surface},{surface},{surface}")
                    }
                    _ => format!("名詞,一般,*,*,*,*,{surface},{surface},{surface}"),
                };
                LatticeMorpheme::new(*surface, features)
            })
            .collect()
    }

    /// The ends of the sentences of a text.
    fn ends(surfaces: &[&str]) -> Vec<usize> {
        sentence_ends(&Schema::ipadic(), &morphemes(surfaces))
    }

    #[test]
    fn no_morpheme_is_no_sentence() {
        assert_eq!(ends(&[]), Vec::<usize>::new());
    }

    #[test]
    fn a_period_ends_a_sentence_and_the_rest_is_one() {
        assert_eq!(ends(&["本", "。"]), [2]);
        assert_eq!(ends(&["本", "。", "本", "。"]), [2, 4]);
        // Without a period at the end.
        assert_eq!(ends(&["本", "．", "本"]), [2, 3]);
        assert_eq!(ends(&["本", "。。", "本"]), [2, 3]);
        assert_eq!(ends(&["本", "本"]), [2]);
        // A period alone is a sentence.
        assert_eq!(ends(&["。"]), [1]);
    }

    #[test]
    fn a_period_of_a_number_and_a_half_width_period_end_nothing() {
        // The half-width period: 4.20, an ellipsis, and at the end.
        assert_eq!(ends(&["4", ".", "20", "本", "。", "本"]), [5, 6]);
        assert_eq!(ends(&["本", ".", ".", ".", "本", "."]), [6]);
        // The full-width one before a digit, of either width.
        assert_eq!(ends(&["３", "．", "１４", "本", "．", "本"]), [5, 6]);
        assert_eq!(ends(&["3", "．", "14", "．"]), [4]);
    }

    #[test]
    fn a_period_inside_brackets_ends_nothing() {
        assert_eq!(ends(&["「", "本", "。", "」", "本", "。", "本"]), [6, 7]);
        // Two levels.
        assert_eq!(
            ends(&["（", "「", "本", "。", "」", "本", "。", "）", "本", "。"]),
            [10]
        );
    }

    #[test]
    fn exclamation_and_question_marks_end_a_sentence() {
        assert_eq!(
            ends(&["本", "！", "本", "？", "本", "!", "本", "?", "本"]),
            [2, 4, 6, 8, 9]
        );
        assert_eq!(ends(&["本", "！？", "本"]), [2, 3]);
        assert_eq!(ends(&["「", "本", "！", "」", "本"]), [5]);
    }

    #[test]
    fn what_follows_an_end_and_belongs_to_it_stays_with_the_sentence() {
        // More ends, and a closing bracket that nothing opened.
        assert_eq!(ends(&["本", "。", "。", "本"]), [3, 4]);
        assert_eq!(ends(&["本", "！", "？", "本"]), [3, 4]);
        assert_eq!(ends(&["本", "。", "」", "本", "。"]), [3, 5]);
    }

    #[test]
    fn a_bracket_that_is_never_closed_keeps_the_rest_in_one_sentence() {
        assert_eq!(ends(&["「", "本", "。", "本", "。"]), [5]);
    }
}

#[cfg(all(test, feature = "embed-ipadic"))]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use lindera_dependency::chunker::{ChunkerModel, Column, Template};
    use lindera_dependency::model::{Model, ModelBuilder};
    use lindera_dependency::parser::{FeatureSet, PairWeights, ParserModel, Quantization};
    use lindera_dependency::{DictionaryMorpheme, Schema};
    use lindera_segmenter::dictionary::load_dictionary;
    use lindera_segmenter::mode::Mode;
    use lindera_segmenter::segmenter::Segmenter;
    use lindera_segmenter::token::{BunsetsuRole, Token, TokenDependency};
    use serde_json::json;

    use super::*;
    use crate::token_filter::{BoxTokenFilter, TokenFilterLoader};
    use crate::tokenizer::Tokenizer;

    /// A chunker that reads the part of speech of a morpheme alone:
    /// particles, auxiliaries and symbols but an opening bracket continue a
    /// bunsetsu, anything else starts one.
    fn chunker() -> ChunkerModel {
        let inside = [
            "助詞-係助詞",
            "助詞-格助詞-一般",
            "助詞-格助詞-引用",
            "助詞-連体化",
            "助動詞",
            "記号-句点",
            "記号-読点",
            "記号-一般",
            "記号-括弧閉",
        ];
        let begin = [
            "名詞-代名詞-一般",
            "名詞-一般",
            "名詞-固有名詞-人名-名",
            "動詞-自立",
            "記号-括弧開",
        ];
        let mut features = HashMap::new();
        for name in inside {
            features.insert(format!("U00:{name}"), [0.0, 1.0]);
        }
        for name in begin {
            features.insert(format!("U00:{name}"), [1.0, 0.0]);
        }
        ChunkerModel::new(
            Schema::ipadic(),
            vec![Column::Surface, Column::PartOfSpeech],
            Template::parse("U00:%x[0,1]\nB\n").unwrap(),
            [[0.0; 3]; 3],
            features,
        )
        .unwrap()
    }

    /// A parser that gives one score to every decision: above 0, a bunsetsu
    /// depends on the next one; below, on the last one.
    fn parser(score: f64) -> ParserModel {
        ParserModel::new(
            Schema::ipadic(),
            FeatureSet::cabocha(),
            score,
            Vec::new(),
            PairWeights::Exact(Vec::new()),
        )
        .unwrap()
    }

    /// The bytes of a model of a dictionary, with that chunker and parser.
    fn model_bytes(dictionary: &str, score: f64) -> Vec<u8> {
        ModelBuilder::new(dictionary, Schema::ipadic())
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .parser(&parser(score), Quantization::F32)
            .unwrap()
            .to_bytes()
    }

    /// The filter with the model of IPADIC.
    fn filter(score: f64) -> JapaneseDependencyTokenFilter {
        JapaneseDependencyTokenFilter::from_bytes(model_bytes("ipadic", score)).unwrap()
    }

    /// A tokenizer of IPADIC without a filter.
    fn tokenizer() -> Tokenizer {
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        Tokenizer::new(Segmenter::new(Mode::Normal, dictionary, None))
    }

    /// What the filter wrote on each token, with its surface.
    fn written<'t>(tokens: &'t [Token<'_>]) -> Vec<(&'t str, Option<&'t TokenDependency>)> {
        tokens
            .iter()
            .map(|token| (token.surface.as_ref(), token.dependency()))
            .collect()
    }

    /// A dependency, for an expected value.
    fn dependency(
        sentence: u32,
        bunsetsu: u32,
        head: Option<u32>,
        role: BunsetsuRole,
        score: f64,
    ) -> TokenDependency {
        TokenDependency {
            sentence,
            bunsetsu,
            head,
            role,
            score,
        }
    }

    /// What the model gives for the tokens of one sentence, read from
    /// copies of their surfaces and details: for each token its bunsetsu,
    /// the head of that bunsetsu, and whether the token is the head word
    /// and the function word.
    fn analyzed(
        model: &Model<Vec<u8>>,
        tokens: &mut [Token<'_>],
    ) -> Vec<(u32, Option<u32>, bool, bool)> {
        let morphemes: Vec<DictionaryMorpheme> = tokens
            .iter_mut()
            .map(|token| {
                let surface = token.surface.to_string();
                let details = token.details().iter().map(|one| one.to_string()).collect();
                DictionaryMorpheme::new(surface, details)
            })
            .collect();
        let mut result = Vec::new();
        for (number, one) in model.analyze(&morphemes).unwrap().iter().enumerate() {
            let (head_word, function_word) = one.positions.unwrap();
            for index in one.morphemes.clone() {
                result.push((
                    number as u32,
                    one.head.map(|head| head as u32),
                    index == one.morphemes.start + head_word,
                    index == one.morphemes.start + function_word,
                ));
            }
        }
        result
    }

    /// A model file in a directory of its own, removed when dropped.
    struct ModelFile {
        directory: tempfile::TempDir,
    }

    impl ModelFile {
        fn new(bytes: &[u8]) -> Self {
            let directory = tempfile::tempdir().unwrap();
            fs::write(directory.path().join("model.bin"), bytes).unwrap();
            Self { directory }
        }

        fn path(&self) -> PathBuf {
            self.directory.path().join("model.bin")
        }
    }

    #[test]
    fn a_sentence_gets_its_bunsetsu_heads_and_roles() {
        use BunsetsuRole::{Function, Head, HeadAndFunction, Other};

        let tokenizer = tokenizer();
        let mut tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        let before: Vec<String> = tokens.iter().map(|one| one.surface.to_string()).collect();
        // Every bunsetsu depends on the next one.
        filter(1.0).apply(&mut tokens).unwrap();
        assert_eq!(
            written(&tokens),
            [
                ("彼", Some(&dependency(0, 0, Some(1), Head, 1.0))),
                ("は", Some(&dependency(0, 0, Some(1), Function, 1.0))),
                ("本", Some(&dependency(0, 1, Some(2), Head, 0.0))),
                ("を", Some(&dependency(0, 1, Some(2), Function, 0.0))),
                ("読ん", Some(&dependency(0, 2, None, Head, 0.0))),
                ("だ", Some(&dependency(0, 2, None, Function, 0.0))),
                ("。", Some(&dependency(0, 2, None, Other, 0.0))),
            ]
        );
        // Nothing else of a token changes.
        let after: Vec<String> = tokens.iter().map(|one| one.surface.to_string()).collect();
        assert_eq!(after, before);
        for (index, token) in tokens.iter().enumerate() {
            assert_eq!((token.position, token.position_length), (index, 1));
        }

        // Every bunsetsu depends on the last one: the heads are the
        // parser's.
        let mut tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        filter(-1.0).apply(&mut tokens).unwrap();
        let heads: Vec<Option<u32>> = tokens
            .iter()
            .map(|token| token.dependency().unwrap().head)
            .collect();
        assert_eq!(
            heads,
            [Some(2), Some(2), Some(2), Some(2), None, None, None]
        );

        // A bunsetsu of one word: it is the head word and the function
        // word.
        let mut tokens = tokenizer.tokenize("本").unwrap();
        filter(1.0).apply(&mut tokens).unwrap();
        assert_eq!(
            written(&tokens),
            [("本", Some(&dependency(0, 0, None, HeadAndFunction, 0.0)))]
        );
    }

    #[test]
    fn the_result_is_what_the_model_gives_for_each_sentence() {
        let filter = filter(1.0);
        let tokenizer = tokenizer();
        let text = "彼は本を読んだ。「これは本です。」と太郎は言った！彼の本";
        let mut tokens = tokenizer.tokenize(text).unwrap();
        filter.apply(&mut tokens).unwrap();

        // Three sentences, the last one without an end.
        let sentences: Vec<u32> = tokens
            .iter()
            .map(|token| token.dependency().unwrap().sentence)
            .collect();
        let surfaces = |sentence: u32| -> String {
            tokens
                .iter()
                .filter(|token| token.dependency().unwrap().sentence == sentence)
                .map(|token| token.surface.as_ref())
                .collect()
        };
        assert_eq!(sentences.iter().max(), Some(&2));
        assert!(sentences.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(surfaces(0), "彼は本を読んだ。");
        assert_eq!(surfaces(1), "「これは本です。」と太郎は言った！");
        assert_eq!(surfaces(2), "彼の本");

        // Each sentence is numbered from 0, and is what the model gives for
        // its tokens alone.
        let mut start = 0;
        for sentence in 0..3 {
            let end = start + sentences.iter().filter(|one| **one == sentence).count();
            let mut alone = tokenizer.tokenize(text).unwrap();
            let expected = analyzed(&filter.model, &mut alone[start..end]);
            assert_eq!(expected[0].0, 0);
            let actual: Vec<(u32, Option<u32>, bool, bool)> = tokens[start..end]
                .iter()
                .map(|token| {
                    let one = token.dependency().unwrap();
                    (
                        one.bunsetsu,
                        one.head,
                        matches!(one.role, BunsetsuRole::Head | BunsetsuRole::HeadAndFunction),
                        matches!(
                            one.role,
                            BunsetsuRole::Function | BunsetsuRole::HeadAndFunction
                        ),
                    )
                })
                .collect();
            assert_eq!(actual, expected, "sentence {sentence}");
            start = end;
        }
        assert_eq!(start, tokens.len());
    }

    #[test]
    fn no_token_and_symbols_alone_are_analyzed_without_an_error() {
        let filter = filter(1.0);
        let mut none: Vec<Token<'_>> = Vec::new();
        filter.apply(&mut none).unwrap();
        assert!(none.is_empty());

        let tokenizer = tokenizer();
        for text in ["。", "。。", "！？", "「」", "、"] {
            let mut tokens = tokenizer.tokenize(text).unwrap();
            filter.apply(&mut tokens).unwrap();
            assert!(!tokens.is_empty(), "{text}");
            assert!(
                tokens.iter().all(|token| token.dependency().is_some()),
                "{text}"
            );
        }
    }

    #[test]
    fn a_model_of_another_dictionary_is_refused() {
        let filter = JapaneseDependencyTokenFilter::from_bytes(model_bytes("unidic", 1.0)).unwrap();
        assert_eq!(filter.dictionary(), "unidic");
        let tokenizer = tokenizer();
        let mut tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        let error = filter.apply(&mut tokens).unwrap_err();
        assert_eq!(error.kind(), LinderaErrorKind::Dictionary);
        let message = error.to_string();
        assert!(message.contains("\"unidic\""), "{message}");
        assert!(message.contains("\"ipadic\""), "{message}");
        // Nothing is written.
        assert!(tokens.iter().all(|token| token.dependency().is_none()));
    }

    #[test]
    fn a_token_with_fewer_details_than_the_model_reads_is_refused() {
        let tokenizer = tokenizer();
        let mut tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        // The schema of IPADIC reads the conjugation in the sixth column.
        tokens[2].details = Some(vec![Cow::Borrowed("名詞"), Cow::Borrowed("一般")]);
        let error = filter(1.0).apply(&mut tokens).unwrap_err();
        assert_eq!(error.kind(), LinderaErrorKind::Dictionary);
        assert!(tokens.iter().all(|token| token.dependency().is_none()));
    }

    #[test]
    fn tokens_removed_or_joined_by_an_earlier_filter_are_refused() {
        let filter = filter(1.0);
        let tokenizer = tokenizer();
        let join = json!({"tags": ["名詞,数"], "new_tag": "名詞,数"});
        let earlier: [(&str, serde_json::Value, &str); 3] = [
            // Removes the particles: a gap in the positions.
            (
                "japanese_stop_tags",
                json!({"tags": ["助詞,係助詞"]}),
                "彼は１０冊読んだ。",
            ),
            // Joins the numbers: a token of two positions, and a gap after
            // it.
            ("japanese_compound_word", join.clone(), "彼は１０冊読んだ。"),
            // The same at the end of the text, where no gap follows.
            ("japanese_compound_word", join, "彼は１０"),
        ];
        for (kind, args, text) in earlier {
            let mut tokens = tokenizer.tokenize(text).unwrap();
            let count = tokens.len();
            TokenFilterLoader::load_from_value(kind, &args)
                .unwrap()
                .apply(&mut tokens)
                .unwrap();
            assert!(tokens.len() < count, "{kind} {text}");
            let error = filter.apply(&mut tokens).unwrap_err();
            assert_eq!(error.kind(), LinderaErrorKind::Args, "{kind} {text}");
            let message = error.to_string();
            assert!(message.contains("removed or joined"), "{message}");
            assert!(tokens.iter().all(|token| token.dependency().is_none()));
        }
    }

    #[test]
    fn what_is_not_a_model_with_a_chunker_and_a_parser_is_refused() {
        let message = |bytes: Vec<u8>| {
            JapaneseDependencyTokenFilter::from_bytes(bytes)
                .unwrap_err()
                .to_string()
        };
        // Not a model file.
        assert!(message(b"not a model".to_vec()).contains("model"));
        // One part is missing.
        let builder = || ModelBuilder::new("ipadic", Schema::ipadic()).unwrap();
        let without_parser = builder().chunker(&chunker()).unwrap().to_bytes();
        assert!(message(without_parser).contains("has no parser"));
        let without_chunker = builder()
            .parser(&parser(1.0), Quantization::F32)
            .unwrap()
            .to_bytes();
        assert!(message(without_chunker).contains("has no chunker"));
    }

    #[test]
    fn the_configuration_names_a_model_file() {
        let file = ModelFile::new(&model_bytes("ipadic", 1.0));
        let path = file.path();
        let args = json!({"model": path.to_str().unwrap()});
        let filter: BoxTokenFilter =
            TokenFilterLoader::load_from_value(JAPANESE_DEPENDENCY_TOKEN_FILTER_NAME, &args)
                .unwrap();
        assert_eq!(filter.name(), "japanese_dependency");

        // In a tokenizer, the JSON of a token has the dependency.
        let mut tokenizer = tokenizer();
        tokenizer.append_token_filter(filter);
        let mut tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        assert_eq!(
            tokens[0].as_value()["dependency"],
            json!({"sentence": 0, "bunsetsu": 0, "head": 1, "role": "head", "score": 1.0})
        );
        assert_eq!(
            tokens[6].as_value()["dependency"],
            json!({"sentence": 0, "bunsetsu": 2, "head": null, "role": "other", "score": 0.0})
        );

        // A file that is not there, a file that is not a model, and no path.
        let missing = file.directory.path().join("missing.bin");
        let error = JapaneseDependencyTokenFilter::from_path(&missing).unwrap_err();
        assert_eq!(error.kind(), LinderaErrorKind::Io);
        assert!(error.to_string().contains("missing.bin"), "{error}");
        fs::write(&missing, b"not a model").unwrap();
        let error = JapaneseDependencyTokenFilter::from_path(&missing).unwrap_err();
        assert!(format!("{error:?}").contains("missing.bin"), "{error:?}");
        for args in [json!({}), json!({"model": 1}), json!(null)] {
            let error = JapaneseDependencyTokenFilter::from_config(&args).unwrap_err();
            assert_eq!(error.kind(), LinderaErrorKind::Deserialize, "{args}");
            assert!(error.to_string().contains("`model`"), "{error}");
        }
    }

    /// The model file the tests of the other crates read (the command, the
    /// bindings): the model of these tests, with every bunsetsu depending
    /// on the next one.
    const MODEL_FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../resources/dependency/test_model_ipadic.bin"
    );

    #[test]
    fn the_model_file_of_the_other_tests_is_the_model_of_these() {
        // When this fails after a change of the format of a model file,
        // write the file again:
        // cargo test -p lindera-analysis --features embed-ipadic,dependency \
        //     --lib write_the_model_file -- --ignored
        assert_eq!(fs::read(MODEL_FILE).unwrap(), model_bytes("ipadic", 1.0));
    }

    #[test]
    #[ignore = "writes resources/dependency/test_model_ipadic.bin"]
    fn write_the_model_file_of_the_other_tests() {
        fs::write(MODEL_FILE, model_bytes("ipadic", 1.0)).unwrap();
    }

    #[test]
    fn a_configuration_file_names_the_filter() {
        use crate::tokenizer::TokenizerBuilder;

        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("lindera.yml");
        fs::write(
            &config,
            format!(
                "segmenter:\n  mode: \"normal\"\n  dictionary: \"embedded://ipadic\"\n\
                 token_filters:\n  - kind: \"japanese_dependency\"\n    args:\n      model: \"{MODEL_FILE}\"\n\
                 \x20 - kind: \"japanese_stop_tags\"\n    args:\n      tags:\n        - \"助詞,係助詞\"\n"
            ),
        )
        .unwrap();
        let tokenizer = TokenizerBuilder::from_file(&config)
            .unwrap()
            .build()
            .unwrap();
        // A filter after this one removes は; what is left keeps its
        // dependency.
        let tokens = tokenizer.tokenize("彼は本を読んだ。").unwrap();
        assert_eq!(
            written(&tokens)[..2],
            [
                (
                    "彼",
                    Some(&dependency(0, 0, Some(1), BunsetsuRole::Head, 1.0))
                ),
                (
                    "本",
                    Some(&dependency(0, 1, Some(2), BunsetsuRole::Head, 0.0))
                ),
            ]
        );
    }

    #[test]
    fn the_clones_of_a_filter_share_the_model() {
        let filter = filter(1.0);
        let clone = filter.clone();
        assert!(Arc::ptr_eq(&filter.model, &clone.model));
    }
}

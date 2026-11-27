//! Converting the models of CaboCha.
//!
//! CaboCha comes with models trained on the Kyoto University Text Corpus,
//! which cannot be trained again without the corpus. This module reads the
//! two text files of such a model, `chunk.ipa.txt` and `dep.ipa.txt`, and
//! makes a model of this crate that analyzes as CaboCha does with them:
//!
//! - the chunker model is a CRF++ model in text: its template, its feature
//!   strings and its weights go into a [`ChunkerModel`] as they are;
//! - the parser model is the support vectors of a kernel of degree 2. They
//!   are expanded into weights of features and of pairs of features, as
//!   `cabocha-model-index` expands them, with its approximation;
//! - what makes the features is data of the model: [`schema`] is CaboCha's
//!   rule for the head and the function word of a bunsetsu, its
//!   punctuation and its brackets, and its normalization of the surface.
//!
//! # The terms of CaboCha's models
//!
//! The license of CaboCha does not cover its models. They were made from
//! the Mainichi Shimbun article data, and CaboCha's page limits the use of
//! the distributed models as they are to research and personal use. A
//! converted model is under the same terms: convert the files of your own
//! copy of CaboCha, and do not distribute the result. This crate holds no
//! model of CaboCha.
//!
//! # Which models
//!
//! The models for the part-of-speech set of IPA, in UTF-8. The models for
//! JUMAN and for UniDic are refused: their rule for the head of a bunsetsu
//! and their columns are other ones.
//!
//! # Examples
//!
//! ```no_run
//! use lindera_dependency::cabocha::{Options, convert};
//! use lindera_dependency::model::Model;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let chunker = std::fs::read_to_string("chunk.ipa.txt")?;
//! let parser = std::fs::read_to_string("dep.ipa.txt")?;
//! let converted = convert(&chunker, &parser, &Options::default())?;
//! println!("{} pairs of {}", converted.report.kept_pairs, converted.report.all_pairs);
//! let model = Model::new(converted.to_bytes()?)?;
//! # Ok(())
//! # }
//! ```

mod crfpp;
mod svm;

use crate::chunker::{ChunkerModel, Column, Template};
use crate::error::{Error, Result};
use crate::model::ModelBuilder;
use crate::parser::{FeatureSet, PairWeights, ParserModel, Quantization};
use crate::schema::{Marks, Normalization, Pattern, Schema, SurfaceRule, Symbols};

/// The table of `src/normalizer.rule` of CaboCha: a line is what is
/// replaced, a tab, and what it is replaced by.
const NORMALIZER_RULE: &str = include_str!("cabocha/normalizer.rule");

/// The part-of-speech set of the dictionary a model of CaboCha is for:
/// `posset` in the header of its parser model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartOfSpeechSet {
    /// The set of IPADIC: `IPA`.
    Ipa,
}

impl PartOfSpeechSet {
    /// The set with the name CaboCha gives it.
    ///
    /// # Arguments
    ///
    /// * `name` - The value of `posset`: `IPA`.
    ///
    /// # Returns
    ///
    /// The set, or [`Error::Cabocha`] for `JUMAN` and `UNIDIC`, which are
    /// not supported, and for any other name.
    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "IPA" => Ok(PartOfSpeechSet::Ipa),
            "JUMAN" | "UNIDIC" => Err(Error::Cabocha(format!(
                "the model is for the part-of-speech set {name}, which is not supported: only the models for IPA are converted"
            ))),
            other => Err(Error::Cabocha(format!(
                "the model is for the part-of-speech set {other:?}, which is not one of CaboCha"
            ))),
        }
    }

    /// The name of the dictionary of Lindera with this set: the name a
    /// converted model is for.
    ///
    /// # Returns
    ///
    /// `ipadic` for [`PartOfSpeechSet::Ipa`].
    pub fn dictionary(self) -> &'static str {
        match self {
            PartOfSpeechSet::Ipa => "ipadic",
        }
    }
}

/// How the pairs of features of the parser model are pruned: the options
/// of `cabocha-model-index`, with its defaults.
///
/// A pair of two of the `frequent_features` most frequent features is
/// always kept. Another pair is kept when it is in `min_support` support
/// vectors or more, and its weight is `sigma × positives / all` or more,
/// or `-sigma × negatives / all` or less.
///
/// `sigma` 0 with `min_support` 1 keeps every pair: the kernel itself,
/// which is not what CaboCha parses with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// The threshold of the weight of a pair: `-s` of
    /// `cabocha-model-index`, 0.0001.
    pub sigma: f64,
    /// The number of support vectors a pair has to be in: `-n`, 2.
    pub min_support: usize,
    /// The number of frequent features, whose pairs are all kept: `-F`,
    /// 3000.
    pub frequent_features: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            sigma: 0.0001,
            min_support: 2,
            frequent_features: 3000,
        }
    }
}

/// What a conversion did.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The number of feature strings of the chunker model.
    pub chunker_features: usize,
    /// The number of support vectors of the parser model.
    pub support_vectors: usize,
    /// The number of support vectors with a weight above 0.
    pub positives: usize,
    /// The number of feature strings of the parser model.
    pub parser_features: usize,
    /// The number of pairs of features in the support vectors.
    pub all_pairs: usize,
    /// The number of pairs the pruning keeps.
    pub kept_pairs: usize,
    /// The number of pairs of the converted model: the kept ones whose
    /// weight is not truncated to 0.
    pub pairs: usize,
    /// What the whole-number weights are multiplied by: CaboCha's
    /// normalize factor.
    pub unit: f64,
}

/// A model of CaboCha, converted.
#[derive(Debug, Clone)]
pub struct Converted {
    /// The part-of-speech set of the model.
    pub part_of_speech_set: PartOfSpeechSet,
    /// The chunker.
    pub chunker: ChunkerModel,
    /// The parser.
    pub parser: ParserModel,
    /// What the conversion did.
    pub report: Report,
}

impl Converted {
    /// The model as the bytes of a model file, for the dictionary of the
    /// part-of-speech set. The weights of the parser are not quantized:
    /// they are the whole numbers CaboCha adds up.
    ///
    /// # Returns
    ///
    /// The bytes, or the error of a model that cannot be written.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(ModelBuilder::new(
            self.part_of_speech_set.dictionary(),
            self.chunker.schema().clone(),
        )?
        .chunker(&self.chunker)?
        .parser(&self.parser, Quantization::F32)?
        .to_bytes())
    }
}

/// CaboCha's table for the normalization of a surface.
///
/// # Returns
///
/// The table of `src/normalizer.rule` of CaboCha: the half-width katakana
/// in full width, the full-width letters, digits and signs in ASCII, the
/// spaces as `___`.
pub fn normalization() -> Normalization {
    let rules: Vec<(&str, &str)> = NORMALIZER_RULE
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    Normalization::new(&rules).expect("the table of CaboCha is a table")
}

/// The schema CaboCha has for a part-of-speech set: what makes the
/// features of its models.
///
/// For IPA (`src/selector_pat.h` of CaboCha): the function words are 助詞,
/// 助動詞, 動詞,非自立, 動詞,接尾, 形容詞,非自立 and 形容詞,接尾; the
/// head is the last morpheme that is neither one of them nor a 記号; the
/// punctuation is `。` and `、`, and the brackets are ten opening and ten
/// closing ones, each matched as the whole normalized surface. It is not
/// [`Schema::ipadic`], which has more patterns, for the bunsetsu of UD
/// Japanese-GSD.
///
/// # Arguments
///
/// * `part_of_speech_set` - The set.
///
/// # Returns
///
/// The schema.
pub fn schema(part_of_speech_set: PartOfSpeechSet) -> Schema {
    match part_of_speech_set {
        PartOfSpeechSet::Ipa => Schema {
            part_of_speech: 0..4,
            conjugation_type: Some(4),
            conjugation_form: Some(5),
            base_form: Some(6),
            function_words: vec![
                Pattern::new(&["助詞"]),
                Pattern::new(&["助動詞"]),
                Pattern::new(&["動詞", "非自立"]),
                Pattern::new(&["動詞", "接尾"]),
                Pattern::new(&["形容詞", "非自立"]),
                Pattern::new(&["形容詞", "接尾"]),
            ],
            not_heads: Vec::new(),
            particles: vec![Pattern::new(&["助詞"])],
            lexicalized: vec![
                Pattern::new(&["助詞"]),
                Pattern::new(&["副詞"]),
                Pattern::new(&["連体詞"]),
                Pattern::new(&["接続詞"]),
            ],
            symbols: Symbols {
                others: Marks::new(&[&["記号"]], ""),
                commas: Marks::new(&[], "、"),
                periods: Marks::new(&[], "。"),
                open_brackets: Marks::new(&[], "(（‘“《「『［〈｛"),
                close_brackets: Marks::new(&[], ")）’”》」』］〉｝"),
                surface: SurfaceRule::OneCharacter,
            },
            normalization: normalization(),
        },
    }
}

/// Reads a value of the header of the parser model, and checks it.
fn check_header(model: &svm::SvmModel, key: &str, accepted: &[&str]) -> Result<()> {
    let value = model
        .header
        .get(key)
        .ok_or_else(|| Error::Cabocha(format!("the parser model has no `{key}` in its header")))?;
    if accepted
        .iter()
        .any(|one| one.eq_ignore_ascii_case(value.trim()))
    {
        Ok(())
    } else {
        Err(Error::Cabocha(format!(
            "the parser model has `{key}: {value}`, and only {} is supported",
            accepted.join(" or ")
        )))
    }
}

/// Converts a model of CaboCha.
///
/// # Arguments
///
/// * `chunker` - The content of the chunker model in text, as
///   `chunk.ipa.txt`.
/// * `parser` - The content of the parser model in text, as `dep.ipa.txt`.
/// * `options` - How the pairs of the parser model are pruned.
///
/// # Returns
///
/// The converted model, or [`Error::Cabocha`] for files that are not the
/// two text models of CaboCha, for a model that is not for IPA in UTF-8
/// with a kernel of degree 2, or for options that are not ones: a `sigma`
/// that is not a finite number of 0 or more.
pub fn convert(chunker: &str, parser: &str, options: &Options) -> Result<Converted> {
    if !options.sigma.is_finite() || options.sigma < 0.0 {
        return Err(Error::Cabocha(format!(
            "sigma is {}, and has to be 0 or more",
            options.sigma
        )));
    }

    let support = svm::read(parser)?;
    let name = support.header.get("posset").ok_or_else(|| {
        Error::Cabocha("the parser model has no `posset` in its header".to_string())
    })?;
    let part_of_speech_set = PartOfSpeechSet::from_name(name.trim())?;
    check_header(&support, "degree", &["2"])?;
    check_header(&support, "charset", &["UTF8", "UTF-8"])?;
    if let Some(kind) = support.header.get("type")
        && kind.trim() != "dep"
    {
        return Err(Error::Cabocha(format!(
            "the parser model has `type: {kind}`, and a parser model has `dep`"
        )));
    }
    let bias = support
        .header
        .get("bias")
        .and_then(|bias| bias.trim().parse::<f64>().ok());
    if bias != Some(0.0) {
        return Err(Error::Cabocha(
            "the parser model has no `bias: 0` in its header, and only a model without a bias is supported".to_string(),
        ));
    }
    let schema = schema(part_of_speech_set);

    let crf = crfpp::read(chunker)?;
    if crf.columns != 2 {
        return Err(Error::Cabocha(format!(
            "the chunker model has {} columns, and the chunker of CaboCha has 2: the surface and the part of speech",
            crf.columns
        )));
    }
    let chunker_features = crf.features.len();
    let chunker = ChunkerModel::new(
        schema.clone(),
        vec![Column::Surface, Column::PartOfSpeech],
        Template::parse(&crf.template)?,
        crf.transitions,
        crf.features,
    )?;

    let expansion = svm::expand(&support, options)?;
    // The features by their strings, which is the order of a model; a
    // number of CaboCha without a string is in no support vector.
    let mut order: Vec<u32> = (0..support.names.len() as u32)
        .filter(|number| !support.names[*number as usize].is_empty())
        .collect();
    order.sort_unstable_by_key(|number| support.names[*number as usize].as_bytes());
    let mut numbers = vec![u32::MAX; support.names.len()];
    for (index, number) in order.iter().enumerate() {
        numbers[*number as usize] = index as u32;
    }
    let features: Vec<(String, f32)> = order
        .iter()
        .map(|number| {
            let weight = expansion
                .weights
                .get(*number as usize)
                .copied()
                .unwrap_or(0);
            (support.names[*number as usize].clone(), weight as f32)
        })
        .collect();
    let mut pairs: Vec<((u32, u32), f32)> = expansion
        .pairs
        .iter()
        .map(|((one, other), weight)| {
            let (one, other) = (numbers[*one as usize], numbers[*other as usize]);
            ((one.min(other), one.max(other)), *weight as f32)
        })
        .collect();
    pairs.sort_unstable_by_key(|(pair, _)| *pair);
    let unit = f64::from(expansion.unit);
    let report = Report {
        chunker_features,
        support_vectors: support.vectors.len(),
        positives: expansion.positives,
        parser_features: features.len(),
        all_pairs: expansion.all_pairs,
        kept_pairs: expansion.kept_pairs,
        pairs: pairs.len(),
        unit,
    };
    // CaboCha starts a decision from the bias and adds whole numbers; the
    // bias of a model is not in the unit of its weights.
    let parser = ParserModel::new(
        schema,
        FeatureSet::cabocha(),
        f64::from(expansion.bias) * unit,
        features,
        PairWeights::Exact(pairs),
    )?
    .with_unit(unit)?;
    Ok(Converted {
        part_of_speech_set,
        chunker,
        parser,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::LatticeMorpheme;
    use crate::model::Model;
    use crate::parser::Classifier;

    #[test]
    fn the_table_is_the_one_of_cabocha() {
        let table = normalization();
        // 182 lines, one of them twice.
        assert_eq!(NORMALIZER_RULE.lines().count(), 182);
        assert_eq!(table.rules().count(), 181);
        for (surface, expected) in [
            ("ｶﾞｲﾄﾞ", "ガイド"),
            ("ＡＢＣ１２３", "ABC123"),
            ("（株）", "(株)"),
            ("｢ｦ｣", "「ヲ」"),
            ("ｳﾞ", "ヴ"),
            ("　", "___"),
            (" ", "___"),
            ("―", "-"),
            ("－", "-"),
            // What the table does not have stays: the full-width comma and
            // the period, the kanji, the hiragana.
            ("，．読ん、。", "，．読ん、。"),
        ] {
            assert_eq!(table.apply(surface), expected, "{surface}");
        }
    }

    #[test]
    fn the_schema_is_the_rule_of_cabocha_for_ipa() {
        let schema = schema(PartOfSpeechSet::Ipa);
        let morpheme = |surface: &str, features: &str| LatticeMorpheme::new(surface, features);
        // The head is the last morpheme that is not a function word or a
        // 記号: する is a head here, and it is not for `Schema::ipadic`.
        let bunsetsu = [
            morpheme("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
            morpheme("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
            morpheme("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
            morpheme("。", "記号,句点,*,*,*,*,。,。,。"),
        ];
        assert_eq!(schema.positions(&bunsetsu), Some((1, 2)));
        assert_eq!(Schema::ipadic().positions(&bunsetsu), Some((0, 2)));
        // A formal noun is a head too.
        let bunsetsu = [
            morpheme("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
            morpheme("こと", "名詞,非自立,一般,*,*,*,こと,コト,コト"),
            morpheme("が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ"),
        ];
        assert_eq!(schema.positions(&bunsetsu), Some((1, 2)));
        // Without a function word after the head, the head is it.
        assert_eq!(schema.positions(&bunsetsu[..2]), Some((1, 1)));

        // Punctuation and brackets by the normalized surface, as a whole.
        let noun = "名詞,サ変接続,*,*,*,*,*";
        assert!(schema.is_comma(&morpheme("、", noun)));
        assert!(schema.is_period(&morpheme("。", noun)));
        assert!(!schema.is_comma(&morpheme(",", noun)));
        assert!(!schema.is_comma(&morpheme("，", noun)));
        assert!(!schema.is_period(&morpheme(".", noun)));
        for open in ["(", "（", "‘", "“", "《", "「", "『", "［", "〈", "｢"] {
            assert!(schema.is_open_bracket(&morpheme(open, noun)), "{open}");
        }
        for close in [")", "）", "》", "」", "』", "］", "〉", "｣"] {
            assert!(schema.is_close_bracket(&morpheme(close, noun)), "{close}");
        }
        for other in ["[", "【", "((", "「「", ""] {
            assert!(!schema.is_open_bracket(&morpheme(other, noun)), "{other}");
        }
        // As in CaboCha, the table makes `｛`, `｝`, `’` and `”` the
        // characters of ASCII before they are matched, and those are not
        // among its brackets.
        assert!(!schema.is_open_bracket(&morpheme("｛", noun)));
        assert!(!schema.is_open_bracket(&morpheme("{", noun)));
        for close in ["｝", "}", "’", "”", "'", "\""] {
            assert!(!schema.is_close_bracket(&morpheme(close, noun)), "{close}");
        }
        // A bracket that is an unknown word is still a word.
        assert!(!schema.is_symbol(&morpheme("(", noun)));
        assert!(schema.is_symbol(&morpheme("x", "記号,一般,*,*,*,*,*")));
        assert!(schema.is_particle(&[morpheme("が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ")], 0));
        assert_eq!(schema.width(), 7);
    }

    #[test]
    fn converts_the_two_models_into_one() {
        let converted = convert(
            crfpp::tests::MODEL,
            svm::tests::MODEL,
            &svm::tests::unpruned(),
        )
        .unwrap();
        assert_eq!(converted.part_of_speech_set, PartOfSpeechSet::Ipa);
        assert_eq!(
            converted.report,
            Report {
                chunker_features: 3,
                support_vectors: 4,
                positives: 2,
                parser_features: 5,
                all_pairs: 7,
                kept_pairs: 7,
                pairs: 7,
                unit: f64::from(2.25f32 / 1_048_575.0),
            }
        );

        // The parser scores with the whole numbers of CaboCha: the bias,
        // the features and the pairs of a decision, times the unit.
        let parser = &converted.parser;
        let unit = converted.report.unit;
        let whole = |weight: f32| f64::from((weight / (2.25f32 / 1_048_575.0)) as i32);
        let id = |name: &str| parser.id(name).unwrap();
        assert_eq!(parser.id("DIST:6-"), None);
        assert_eq!(parser.score(&[]), whole(0.375) * unit);
        assert_eq!(
            parser.score(&[id("DHS:読む")]),
            (whole(0.375) + 1_048_575.0) * unit
        );
        let mut both = [id("DIST:1"), id("DHS:読む")];
        both.sort_unstable();
        assert_eq!(
            parser.score(&both),
            (whole(0.375) + whole(0.375) + 1_048_575.0 + whole(1.0)) * unit
        );
        // A negative decision: the kernel gives -0.375.
        let mut negative = [id("DIST:1"), id("GNB:1"), id("SHS:本")];
        negative.sort_unstable();
        let score = parser.score(&negative);
        assert!((score + 0.375).abs() < 8.0 * unit, "{score}");
        assert_eq!(parser.feature_set(), &FeatureSet::cabocha());
        assert_eq!(parser.schema(), &schema(PartOfSpeechSet::Ipa));

        // The file is one for IPADIC, with both parts, and scores as the
        // model in memory.
        let model = Model::new(converted.to_bytes().unwrap()).unwrap();
        assert_eq!(model.dictionary(), "ipadic");
        assert_eq!(model.schema(), &schema(PartOfSpeechSet::Ipa));
        assert!(model.chunker().is_some());
        let part = model.parser().unwrap();
        let classifier = part.classifier();
        for features in [vec![], negative.to_vec(), both.to_vec()] {
            assert_eq!(classifier.score(&features), parser.score(&features));
        }
    }

    #[test]
    fn the_converted_model_analyzes_with_the_features_of_cabocha() {
        // A chunker that starts a bunsetsu at a noun after a particle, and
        // a parser that knows one feature of the source: the normalized
        // surface of its head.
        // After `B` and after `I` the chunker goes on with `I`, unless a
        // noun or a verb follows a particle.
        let chunker = "version: 100\ncost-factor: 1\nmaxid: 8\nxsize: 2\n\nB\nI\n\nU00:%x[0,0]\nU01:%x[-1,1]/%x[0,1]\nB\n\n0 B\n4 U01:助詞-格助詞-一般/名詞-一般\n6 U01:助詞-格助詞-一般/動詞-自立\n\n-1\n1\n-1\n1\n5\n-5\n5\n-5\n";
        let parser = "degree: 2\nbias: 0.000000\ncharset: UTF8\nposset: IPA\n\n0 SHS:ABC\n1 DIST:1\n\n1 0\n-0.25 1\n";
        let converted = convert(chunker, parser, &svm::tests::unpruned()).unwrap();
        let model = Model::new(converted.to_bytes().unwrap()).unwrap();
        let sentence = |first: &str| {
            vec![
                LatticeMorpheme::new(first, "名詞,一般,*,*,*,*,*"),
                LatticeMorpheme::new("が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ"),
                LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
                LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
                LatticeMorpheme::new("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
            ]
        };
        // ＡＢＣ is ABC for the model: its bunsetsu depends on the next.
        let bunsetsu = model.analyze(&sentence("ＡＢＣ")).unwrap();
        let heads: Vec<Option<usize>> = bunsetsu.iter().map(|one| one.head).collect();
        assert_eq!(heads, [Some(1), Some(2), None]);
        // Another word has no positive feature, and waits for the last.
        let bunsetsu = model.analyze(&sentence("彼")).unwrap();
        let heads: Vec<Option<usize>> = bunsetsu.iter().map(|one| one.head).collect();
        assert_eq!(heads, [Some(2), Some(2), None]);
    }

    #[test]
    fn refuses_what_is_not_a_model_for_ipa() {
        let chunker = crfpp::tests::MODEL;
        let parser = svm::tests::MODEL;
        let message = |chunker: &str, parser: &str, options: &Options| {
            convert(chunker, parser, options).unwrap_err().to_string()
        };
        let options = Options::default();
        for (parser, expected) in [
            (
                parser.replace("posset: IPA", "posset: JUMAN"),
                "the part-of-speech set JUMAN, which is not supported",
            ),
            (
                parser.replace("posset: IPA", "posset: UNIDIC"),
                "the part-of-speech set UNIDIC, which is not supported",
            ),
            (
                parser.replace("posset: IPA", "posset: PENN"),
                "the part-of-speech set \"PENN\", which is not one of CaboCha",
            ),
            (parser.replace("posset: IPA\n", ""), "has no `posset`"),
            (
                parser.replace("degree: 2", "degree: 3"),
                "has `degree: 3`, and only 2 is supported",
            ),
            (parser.replace("degree: 2\n", ""), "has no `degree`"),
            (
                parser.replace("charset: UTF8", "charset: EUC-JP"),
                "has `charset: EUC-JP`, and only UTF8 or UTF-8 is supported",
            ),
            (
                parser.replace("type: dep", "type: chunk"),
                "has `type: chunk`",
            ),
            (
                parser.replace("bias: 0.000000", "bias: 0.500000"),
                "has no `bias: 0`",
            ),
            (parser.replace("bias: 0.000000\n", ""), "has no `bias: 0`"),
        ] {
            let message = message(chunker, &parser, &options);
            assert!(message.contains(expected), "{expected}: {message}");
        }
        // The two files the other way round.
        assert!(message(parser, chunker, &options).starts_with("the parser model "));
        assert!(message(chunker, chunker, &options).starts_with("the parser model "));
        assert!(message(parser, parser, &options).starts_with("the chunker model "));
        let three = chunker.replace("xsize: 2", "xsize: 3");
        assert!(message(&three, parser, &options).contains("has 3 columns"));
        for sigma in [-0.1, f64::NAN, f64::INFINITY] {
            let options = Options { sigma, ..options };
            assert!(message(chunker, parser, &options).starts_with("sigma is "));
        }
    }

    #[test]
    fn the_defaults_are_those_of_cabocha_model_index() {
        assert_eq!(
            Options::default(),
            Options {
                sigma: 0.0001,
                min_support: 2,
                frequent_features: 3000
            }
        );
        assert_eq!(PartOfSpeechSet::Ipa.dictionary(), "ipadic");
    }
}

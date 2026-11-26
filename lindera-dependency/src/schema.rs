//! The schema of a dictionary: which feature column holds what, and which
//! morphemes of a bunsetsu are its head and its function word.
//!
//! The feature columns of a morpheme differ from one dictionary to another,
//! and so do the parts of speech that make a function word or a symbol.
//! What needs them (the chunker, the parser, the output) takes them from a
//! [`Schema`], and holds none of its own. A schema is data: a second
//! dictionary is a second value, not a second code path.
//!
//! # The head and the function word
//!
//! The head (主辞) of a bunsetsu is its last content word, and the function
//! word (機能語) its last particle or auxiliary. [`Schema::positions`] finds
//! them as `findHead` of CaboCha does:
//!
//! 1. Both start at the first morpheme of the bunsetsu.
//! 2. From the left, a morpheme that matches one of
//!    [`Schema::function_words`] becomes the function word; a morpheme that
//!    matches none of them, none of [`Schema::not_heads`], and is not a
//!    symbol becomes the head.
//! 3. A head after the function word becomes the function word too.
//!
//! # Examples
//!
//! ```
//! use lindera_dependency::Schema;
//! use lindera_dependency::lattice::LatticeMorpheme;
//!
//! let schema = Schema::ipadic();
//! // 読ん / で / いる / 。
//! let bunsetsu = [
//!     LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
//!     LatticeMorpheme::new("で", "助詞,接続助詞,*,*,*,*,で,デ,デ"),
//!     LatticeMorpheme::new("いる", "動詞,非自立,*,*,一段,基本形,いる,イル,イル"),
//!     LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
//! ];
//! assert_eq!(schema.positions(&bunsetsu), Some((0, 2)));
//! assert!(schema.is_period(&bunsetsu[3]));
//! ```

mod pattern;
mod symbols;

use std::ops::Range;

pub use pattern::Pattern;
pub use symbols::{Marks, Symbols};

use crate::bytes::{Reader, push_u32};
use crate::error::{Error, Result};
use crate::morpheme::Morpheme;

/// The value that stands for a column the schema does not have, in its
/// bytes.
const NO_COLUMN: u32 = u32::MAX;

/// The value of a feature column that holds nothing.
const EMPTY: &str = "*";

/// The characters of the commas, in full width and half width.
/// The formal nouns of the schema of UniDic, by their 語彙素: こと, ため,
/// もの, 際, うえ and ところ.
const FORMAL_NOUNS: [&str; 6] = ["事", "為", "物", "際", "上", "所"];

const COMMAS: &str = "、，,";

/// The characters of the periods, in full width and half width.
const PERIODS: &str = "。．.";

/// The opening brackets: those CaboCha knows, and the other ones of the
/// same blocks.
const OPEN_BRACKETS: &str = "(（[［{｛「『【〔〈《〘〚‘“";

/// The closing brackets, in the order of [`OPEN_BRACKETS`].
const CLOSE_BRACKETS: &str = ")）]］}｝」』】〕〉》〙〛’”";

/// The other characters of symbols: the punctuation of ASCII with its
/// full-width forms, and marks of Japanese text. `%` and `°` are not here:
/// they are units, which UD Japanese-GSD has as the content head (50 / %).
const OTHER_SYMBOLS: &str = concat!(
    "!\"#$&'*+-/:;<=>?@\\^_`|~",
    "！＂＃＄＆＇＊＋－／：；＜＝＞？＠＼＾＿｀｜～",
    "・…‥―—–‐〜※♪★☆○●◎◇◆□■△▲▽▼→←↑↓≠",
);

/// What is known about a dictionary: which feature columns hold the part of
/// speech, the conjugation and the base form, which morphemes are function
/// words, which are not heads, and which are symbols.
///
/// Columns are counted from 0, as [`Morpheme::feature`]
/// does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Schema {
    /// The columns of the part of speech, from the coarsest level to the
    /// finest.
    pub part_of_speech: Range<usize>,
    /// The column of the conjugation type, when the dictionary has one.
    pub conjugation_type: Option<usize>,
    /// The column of the conjugation form, when the dictionary has one.
    pub conjugation_form: Option<usize>,
    /// The column of the base form, when the dictionary has one.
    pub base_form: Option<usize>,
    /// The function words: a morpheme that matches one of these patterns.
    pub function_words: Vec<Pattern>,
    /// What is neither a function word nor a head, besides the symbols: a
    /// morpheme that matches one of these patterns.
    pub not_heads: Vec<Pattern>,
    /// The particles: a morpheme that matches one of these patterns. The
    /// parser names the particles of a bunsetsu in its features.
    pub particles: Vec<Pattern>,
    /// The function words that tell by their surface how a bunsetsu
    /// modifies another: a morpheme that matches one of these patterns. For
    /// a bunsetsu that ends in any other word, the parser takes the
    /// conjugation form or the part of speech in its place.
    pub lexicalized: Vec<Pattern>,
    /// How symbols, punctuation and brackets are recognized.
    pub symbols: Symbols,
}

impl Schema {
    /// The names of the dictionaries [`Schema::for_dictionary`] has a schema
    /// for.
    pub const DICTIONARIES: [&'static str; 4] = ["ipadic", "unidic", "unidic-cwj", "unidic-csj"];

    /// The schema of IPADIC.
    ///
    /// The columns are four levels of part of speech, then the conjugation
    /// type, the conjugation form and the base form.
    ///
    /// The function words are those of CaboCha for IPA (助詞, 助動詞,
    /// 動詞,非自立, 動詞,接尾, 形容詞,非自立, 形容詞,接尾), and する and
    /// できる (動詞,自立): in 利用 / し / た the head is 利用. At the start
    /// of a bunsetsu する is still the head, since the head starts there
    /// and only a head moves it.
    ///
    /// Besides the symbols, these are not heads: 名詞,非自立 (こと, の,
    /// よう); the suffixes 的 (名詞,接尾,形容動詞語幹) and さ
    /// (名詞,接尾,特殊), so that in 基本 / 的 / に the head is 基本; and a
    /// 動詞,自立 right after a 助詞, which is part of a compound function
    /// word (こと / に / なっ / た, かも / しれ / ない).
    ///
    /// With CaboCha's rule alone the head is the content head of UD
    /// Japanese-GSD in 85% of the bunsetsu; with these additions, in 97%.
    /// The README of the crate has the measurement.
    ///
    /// A symbol is a morpheme of the part of speech 記号, or one whose
    /// surface is made of symbol characters only: IPADIC has no half-width
    /// symbols, and gives `,` as 名詞,サ変接続.
    ///
    /// The particles are 助詞, and the function words taken by their
    /// surface are 助詞, 副詞, 連体詞 and 接続詞, as in CaboCha.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn ipadic() -> Self {
        Self {
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
                Pattern::new(&["動詞", "自立"]).with_base_forms(&["する", "できる", "出来る"]),
            ],
            not_heads: vec![
                Pattern::new(&["名詞", "非自立"]),
                Pattern::new(&["名詞", "接尾", "形容動詞語幹"]),
                Pattern::new(&["名詞", "接尾", "特殊"]),
                Pattern::new(&["動詞", "自立"]).after(Pattern::new(&["助詞"])),
            ],
            particles: vec![Pattern::new(&["助詞"])],
            lexicalized: vec![
                Pattern::new(&["助詞"]),
                Pattern::new(&["副詞"]),
                Pattern::new(&["連体詞"]),
                Pattern::new(&["接続詞"]),
            ],
            symbols: Symbols {
                others: Marks::new(&[&["記号"]], OTHER_SYMBOLS),
                commas: Marks::new(&[&["記号", "読点"]], COMMAS),
                periods: Marks::new(&[&["記号", "句点"]], PERIODS),
                open_brackets: Marks::new(&[&["記号", "括弧開"]], OPEN_BRACKETS),
                close_brackets: Marks::new(&[&["記号", "括弧閉"]], CLOSE_BRACKETS),
            },
        }
    }

    /// The schema of UniDic: of `unidic` (unidic-mecab 2.1.2), `unidic-cwj`
    /// and `unidic-csj`, whose first columns are the same.
    ///
    /// The columns are 品詞大分類, 中分類, 小分類, 細分類, 活用型, 活用形,
    /// 語彙素読み and 語彙素; the base form is the 語彙素, so that of こと
    /// is 事.
    ///
    /// The function words are the ones of the pattern CaboCha has for
    /// UniDic (a particle, an auxiliary, and a suffix that is not nominal),
    /// and a verb or an adjective that is 非自立可能. What is not a head
    /// was measured on UD Japanese-GSD, as for IPADIC:
    ///
    /// * a verb right after に, と or を (に / よっ / て, と / し / て),
    /// * a 助動詞語幹 (よう, そう, みたい),
    /// * a formal noun (こと, ため, もの, 際, うえ, ところ) right after a
    ///   verb, an auxiliary or an adjective,
    /// * a nominal suffix right after an adjective (大き / さ).
    ///
    /// UniDic tags a letter of the Latin alphabet 記号,文字, and it is a
    /// symbol here as any 記号 is.
    ///
    /// # Returns
    ///
    /// The schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use lindera_dependency::Schema;
    /// use lindera_dependency::lattice::LatticeMorpheme;
    ///
    /// let schema = Schema::unidic();
    /// // 利用 / する / こと / が
    /// let bunsetsu = [
    ///     LatticeMorpheme::new("利用", "名詞,普通名詞,サ変可能,*,*,*,リヨウ,利用"),
    ///     LatticeMorpheme::new("する", "動詞,非自立可能,*,*,サ行変格,連体形-一般,スル,為る"),
    ///     LatticeMorpheme::new("こと", "名詞,普通名詞,一般,*,*,*,コト,事"),
    ///     LatticeMorpheme::new("が", "助詞,格助詞,*,*,*,*,ガ,が"),
    /// ];
    /// // The head word is 利用 and the function word が: する is 非自立可能,
    /// // and こと is a formal noun after a verb.
    /// assert_eq!(schema.positions(&bunsetsu), Some((0, 3)));
    /// ```
    pub fn unidic() -> Self {
        Self {
            part_of_speech: 0..4,
            conjugation_type: Some(4),
            conjugation_form: Some(5),
            base_form: Some(7),
            function_words: vec![
                Pattern::new(&["助詞"]),
                Pattern::new(&["助動詞"]),
                Pattern::new(&["接尾辞", "形容詞的"]),
                Pattern::new(&["接尾辞", "形状詞的"]),
                Pattern::new(&["接尾辞", "動詞的"]),
                Pattern::new(&["動詞", "非自立可能"]),
                Pattern::new(&["形容詞", "非自立可能"]),
            ],
            not_heads: vec![
                Pattern::new(&["動詞"])
                    .after(Pattern::new(&["助詞"]).with_base_forms(&["に", "と", "を"])),
                Pattern::new(&["形状詞", "助動詞語幹"]),
                Pattern::new(&["名詞", "助動詞語幹"]),
                Pattern::new(&["名詞", "普通名詞"])
                    .with_base_forms(&FORMAL_NOUNS)
                    .after(Pattern::new(&["動詞"])),
                Pattern::new(&["名詞", "普通名詞"])
                    .with_base_forms(&FORMAL_NOUNS)
                    .after(Pattern::new(&["助動詞"])),
                Pattern::new(&["名詞", "普通名詞"])
                    .with_base_forms(&FORMAL_NOUNS)
                    .after(Pattern::new(&["形容詞"])),
                Pattern::new(&["接尾辞", "名詞的"]).after(Pattern::new(&["形容詞"])),
            ],
            particles: vec![Pattern::new(&["助詞"])],
            lexicalized: vec![
                Pattern::new(&["助詞"]),
                Pattern::new(&["副詞"]),
                Pattern::new(&["連体詞"]),
                Pattern::new(&["接続詞"]),
            ],
            symbols: Symbols {
                others: Marks::new(&[&["空白"], &["補助記号"], &["記号"]], ""),
                commas: Marks::new(&[&["補助記号", "読点"]], COMMAS),
                periods: Marks::new(&[&["補助記号", "句点"]], PERIODS),
                open_brackets: Marks::new(&[&["補助記号", "括弧開"]], OPEN_BRACKETS),
                close_brackets: Marks::new(&[&["補助記号", "括弧閉"]], CLOSE_BRACKETS),
            },
        }
    }

    /// The schema of a dictionary of Lindera, by the name of the dictionary.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The name of the dictionary, as its metadata has it:
    ///   `ipadic`, `unidic`, `unidic-cwj` or `unidic-csj`.
    ///
    /// # Returns
    ///
    /// The schema, or `None` for a dictionary this crate has no schema for.
    ///
    /// # Examples
    ///
    /// ```
    /// use lindera_dependency::Schema;
    ///
    /// assert_eq!(Schema::for_dictionary("ipadic"), Some(Schema::ipadic()));
    /// // The three UniDic dictionaries have one schema.
    /// assert_eq!(Schema::for_dictionary("unidic-cwj"), Some(Schema::unidic()));
    /// assert_eq!(Schema::for_dictionary("ko-dic"), None);
    /// assert_eq!(
    ///     Schema::DICTIONARIES,
    ///     ["ipadic", "unidic", "unidic-cwj", "unidic-csj"]
    /// );
    /// ```
    pub fn for_dictionary(dictionary: &str) -> Option<Self> {
        match dictionary {
            "ipadic" => Some(Self::ipadic()),
            "unidic" | "unidic-cwj" | "unidic-csj" => Some(Self::unidic()),
            _ => None,
        }
    }

    /// The positions of the head word and of the function word of a
    /// bunsetsu: what [`Bunsetsu::positions`](crate::Bunsetsu::positions)
    /// holds.
    ///
    /// See the [module documentation](self) for the rule. A bunsetsu of
    /// symbols only has both at its first morpheme; a bunsetsu without a
    /// function word has the function word at its head.
    ///
    /// # Arguments
    ///
    /// * `bunsetsu` - The morphemes of the bunsetsu.
    ///
    /// # Returns
    ///
    /// The positions of the head word and of the function word, counted
    /// from the first morpheme of the bunsetsu; `None` for no morpheme.
    pub fn positions<M: Morpheme>(&self, bunsetsu: &[M]) -> Option<(usize, usize)> {
        if bunsetsu.is_empty() {
            return None;
        }
        let mut head = 0;
        let mut function = 0;
        for (index, morpheme) in bunsetsu.iter().enumerate() {
            let matches = |patterns: &[Pattern]| {
                patterns
                    .iter()
                    .any(|one| one.matches(self, bunsetsu, index))
            };
            if matches(&self.function_words) {
                function = index;
            } else if !matches(&self.not_heads) && !self.is_symbol(morpheme) {
                head = index;
            }
        }
        if head > function {
            function = head;
        }
        Some((head, function))
    }

    /// Whether a morpheme of a bunsetsu is a particle: it matches one of
    /// [`Schema::particles`].
    ///
    /// # Arguments
    ///
    /// * `bunsetsu` - The morphemes of the bunsetsu.
    /// * `index` - The index of the morpheme in the bunsetsu.
    ///
    /// # Returns
    ///
    /// Whether it is a particle; `false` for an index past the bunsetsu.
    pub fn is_particle<M: Morpheme>(&self, bunsetsu: &[M], index: usize) -> bool {
        self.particles
            .iter()
            .any(|one| one.matches(self, bunsetsu, index))
    }

    /// Whether a morpheme of a bunsetsu tells by its surface how the
    /// bunsetsu modifies another: it matches one of
    /// [`Schema::lexicalized`].
    ///
    /// # Arguments
    ///
    /// * `bunsetsu` - The morphemes of the bunsetsu.
    /// * `index` - The index of the morpheme in the bunsetsu.
    ///
    /// # Returns
    ///
    /// Whether it is such a word; `false` for an index past the bunsetsu.
    pub fn is_lexicalized<M: Morpheme>(&self, bunsetsu: &[M], index: usize) -> bool {
        self.lexicalized
            .iter()
            .any(|one| one.matches(self, bunsetsu, index))
    }

    /// Whether a morpheme is a symbol: a comma, a period, a bracket or any
    /// other one. A symbol is neither the head nor the function word of a
    /// bunsetsu.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech is one of a symbol, or its surface is
    /// made of symbol characters only.
    pub fn is_symbol<M: Morpheme>(&self, morpheme: &M) -> bool {
        self.symbols.matches(self, morpheme)
    }

    /// Whether a morpheme is a comma (読点): `、`, or `,` and `，`.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech or its surface is the one of a comma.
    pub fn is_comma<M: Morpheme>(&self, morpheme: &M) -> bool {
        self.symbols.commas.matches(self, morpheme)
    }

    /// Whether a morpheme is a period (句点): `。`, or `.` and `．`.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech or its surface is the one of a period.
    pub fn is_period<M: Morpheme>(&self, morpheme: &M) -> bool {
        self.symbols.periods.matches(self, morpheme)
    }

    /// Whether a morpheme is an opening bracket.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech or its surface is the one of an opening
    /// bracket.
    pub fn is_open_bracket<M: Morpheme>(&self, morpheme: &M) -> bool {
        self.symbols.open_brackets.matches(self, morpheme)
    }

    /// Whether a morpheme is a closing bracket.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech or its surface is the one of a closing
    /// bracket.
    pub fn is_close_bracket<M: Morpheme>(&self, morpheme: &M) -> bool {
        self.symbols.close_brackets.matches(self, morpheme)
    }

    /// The part of speech of a morpheme as one string: its levels joined by
    /// `-`, up to the first one that is `*` or missing, as `名詞-サ変接続`.
    ///
    /// # Arguments
    ///
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// The part of speech; an empty string without a first level.
    pub fn part_of_speech_of<M: Morpheme>(&self, morpheme: &M) -> String {
        let mut value = String::new();
        for index in self.part_of_speech.clone() {
            match morpheme.feature(index) {
                Some(level) if level != EMPTY => {
                    if !value.is_empty() {
                        value.push('-');
                    }
                    value.push_str(level);
                }
                _ => break,
            }
        }
        value
    }

    /// The number of feature columns the schema reads.
    ///
    /// # Returns
    ///
    /// One more than the largest index among the columns of the part of
    /// speech, of the conjugation and of the base form: a morpheme of the
    /// dictionary has at least this many columns.
    pub fn width(&self) -> usize {
        [
            Some(self.part_of_speech.end),
            self.conjugation_type.map(|index| index + 1),
            self.conjugation_form.map(|index| index + 1),
            self.base_form.map(|index| index + 1),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or(0)
    }

    /// Checks that morphemes can be of the dictionary of the schema.
    ///
    /// A morpheme with fewer feature columns than the schema reads comes
    /// from another dictionary, or from a tokenizer that drops columns:
    /// analyzing it would give an answer from columns that are not there.
    /// Morphemes of another dictionary that are wide enough pass; the name
    /// of the dictionary in a model file is for those
    /// ([`Model::check_dictionary`](crate::model::Model::check_dictionary)).
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or [`Error::Dictionary`] for the first morpheme that has
    /// fewer columns than [`Schema::width`].
    pub fn check<M: Morpheme>(&self, morphemes: &[M]) -> Result<()> {
        let width = self.width();
        let Some(last) = width.checked_sub(1) else {
            return Ok(());
        };
        for (index, morpheme) in morphemes.iter().enumerate() {
            if morpheme.feature(last).is_none() {
                return Err(Error::Dictionary(format!(
                    "the morpheme {} ({:?}) has fewer than the {width} feature columns the schema reads: it is not of the dictionary of the model",
                    index + 1,
                    morpheme.surface()
                )));
            }
        }
        Ok(())
    }

    /// Appends the schema to the bytes of a model.
    ///
    /// The columns, then the function word patterns, the patterns of what
    /// is not a head, the particles, the function words taken by their
    /// surface, and the symbols.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    pub(crate) fn write(&self, bytes: &mut Vec<u8>) {
        let column = |index: Option<usize>| index.map_or(NO_COLUMN, |index| index as u32);
        push_u32(bytes, self.part_of_speech.start as u32);
        push_u32(bytes, self.part_of_speech.end as u32);
        push_u32(bytes, column(self.conjugation_type));
        push_u32(bytes, column(self.conjugation_form));
        push_u32(bytes, column(self.base_form));
        for patterns in [
            &self.function_words,
            &self.not_heads,
            &self.particles,
            &self.lexicalized,
        ] {
            push_u32(bytes, patterns.len() as u32);
            for pattern in patterns {
                pattern.write(bytes);
            }
        }
        self.symbols.write(bytes);
    }

    /// Reads a schema from the bytes of a model.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the schema.
    ///
    /// # Returns
    ///
    /// The schema, or the error of bytes that are not one.
    pub(crate) fn read(reader: &mut Reader) -> Result<Self> {
        let start = reader.u32()? as usize;
        let end = reader.u32()? as usize;
        let mut column = || -> Result<Option<usize>> {
            let value = reader.u32()?;
            Ok((value != NO_COLUMN).then_some(value as usize))
        };
        let conjugation_type = column()?;
        let conjugation_form = column()?;
        let base_form = column()?;
        let mut patterns = || -> Result<Vec<Pattern>> {
            let count = reader.u32()?;
            let mut patterns = Vec::new();
            for _ in 0..count {
                patterns.push(Pattern::read(reader, 0)?);
            }
            Ok(patterns)
        };
        let function_words = patterns()?;
        let not_heads = patterns()?;
        let particles = patterns()?;
        let lexicalized = patterns()?;
        Ok(Self {
            part_of_speech: start..end,
            conjugation_type,
            conjugation_form,
            base_form,
            function_words,
            not_heads,
            particles,
            lexicalized,
            symbols: Symbols::read(reader)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::lattice::LatticeMorpheme;

    /// Morphemes of `(surface, features)`.
    fn morphemes(pairs: &[(&str, &str)]) -> Vec<LatticeMorpheme> {
        pairs
            .iter()
            .map(|(surface, features)| LatticeMorpheme::new(*surface, *features))
            .collect()
    }

    /// The positions the schema of IPADIC gives for a bunsetsu.
    fn positions(pairs: &[(&str, &str)]) -> (usize, usize) {
        Schema::ipadic().positions(&morphemes(pairs)).unwrap()
    }

    const HON: (&str, &str) = ("本", "名詞,一般,*,*,*,*,本,ホン,ホン");
    const WO: (&str, &str) = ("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ");
    const NI: (&str, &str) = ("に", "助詞,格助詞,一般,*,*,*,に,ニ,ニ");
    const GA: (&str, &str) = ("が", "助詞,格助詞,一般,*,*,*,が,ガ,ガ");
    const YON: (&str, &str) = ("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン");
    const YOMU: (&str, &str) = ("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム");
    const DA: (&str, &str) = ("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ");
    const TA: (&str, &str) = ("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ");
    const DE: (&str, &str) = ("で", "助詞,接続助詞,*,*,*,*,で,デ,デ");
    const IRU: (&str, &str) = ("いる", "動詞,非自立,*,*,一段,基本形,いる,イル,イル");
    const MARU: (&str, &str) = ("。", "記号,句点,*,*,*,*,。,。,。");
    const TEN: (&str, &str) = ("、", "記号,読点,*,*,*,*,、,、,、");
    const KAGI: (&str, &str) = ("「", "記号,括弧開,*,*,*,*,「,「,「");
    const KAGI_TOJI: (&str, &str) = ("」", "記号,括弧閉,*,*,*,*,」,」,」");
    const RIYOU: (&str, &str) = ("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー");
    const SHI: (&str, &str) = ("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ");
    const KOTO: (&str, &str) = ("こと", "名詞,非自立,一般,*,*,*,こと,コト,コト");
    const KIHON: (&str, &str) = ("基本", "名詞,一般,*,*,*,*,基本,キホン,キホン");
    const TEKI: (&str, &str) = ("的", "名詞,接尾,形容動詞語幹,*,*,*,的,テキ,テキ");
    /// The half-width comma, an unknown word of IPADIC.
    const COMMA: (&str, &str) = (",", "名詞,サ変接続,*,*,*,*,*,*,*");

    #[test]
    fn ipadic_names_the_columns_of_mecab_ipadic() {
        // 読ん: 動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
        let features = [
            "動詞",
            "自立",
            "*",
            "*",
            "五段・マ行",
            "連用タ接続",
            "読む",
            "ヨン",
            "ヨン",
        ];
        let schema = Schema::ipadic();
        assert_eq!(
            features[schema.part_of_speech.clone()],
            ["動詞", "自立", "*", "*"]
        );
        assert_eq!(features[schema.conjugation_type.unwrap()], "五段・マ行");
        assert_eq!(features[schema.conjugation_form.unwrap()], "連用タ接続");
        assert_eq!(features[schema.base_form.unwrap()], "読む");
    }

    #[test]
    fn a_content_word_and_a_particle_are_the_head_and_the_function_word() {
        assert_eq!(positions(&[HON, WO]), (0, 1));
        // The last content word and the last function word.
        assert_eq!(positions(&[KIHON, HON, NI, WO]), (1, 3));
    }

    #[test]
    fn a_content_word_alone_is_both() {
        assert_eq!(positions(&[HON]), (0, 0));
        assert_eq!(positions(&[KIHON, HON]), (1, 1));
    }

    #[test]
    fn punctuation_at_the_end_is_neither() {
        assert_eq!(positions(&[YON, DA, MARU]), (0, 1));
        assert_eq!(positions(&[HON, WO, TEN]), (0, 1));
        // By the surface too: the half-width comma is 名詞,サ変接続.
        assert_eq!(positions(&[HON, WO, COMMA]), (0, 1));
        assert_eq!(positions(&[HON, COMMA]), (0, 0));
    }

    #[test]
    fn a_bunsetsu_of_symbols_only_has_both_at_its_first_morpheme() {
        assert_eq!(positions(&[MARU]), (0, 0));
        assert_eq!(positions(&[KAGI_TOJI, MARU, COMMA]), (0, 0));
    }

    #[test]
    fn a_bracket_before_the_head_is_skipped() {
        assert_eq!(positions(&[KAGI, HON, WO]), (1, 2));
    }

    #[test]
    fn a_dependent_verb_is_the_function_word() {
        // 読ん / で / いる
        assert_eq!(positions(&[YON, DE, IRU]), (0, 2));
        assert_eq!(positions(&[YON, DE, IRU, MARU]), (0, 2));
    }

    #[test]
    fn a_head_after_the_function_word_takes_the_function_word_with_it() {
        // に / 本: a particle, then the content word.
        assert_eq!(positions(&[NI, HON]), (1, 1));
        // The same with a symbol after it.
        assert_eq!(positions(&[NI, HON, MARU]), (1, 1));
        // A function word after the head is kept.
        assert_eq!(positions(&[NI, HON, DA]), (1, 2));
    }

    #[test]
    fn the_function_words_of_cabocha_are_function_words() {
        // 読ま / れる (動詞,接尾)
        let yoma = ("読ま", "動詞,自立,*,*,五段・マ行,未然形,読む,ヨマ,ヨマ");
        let reru = ("れる", "動詞,接尾,*,*,一段,基本形,れる,レル,レル");
        assert_eq!(positions(&[yoma, reru]), (0, 1));
        // 読み / にくい (形容詞,非自立)
        let yomi = ("読み", "動詞,自立,*,*,五段・マ行,連用形,読む,ヨミ,ヨミ");
        let nikui = (
            "にくい",
            "形容詞,非自立,*,*,形容詞・アウオ段,基本形,にくい,ニクイ,ニクイ",
        );
        assert_eq!(positions(&[yomi, nikui]), (0, 1));
        // 本 / っぽい (形容詞,接尾)
        let ppoi = (
            "っぽい",
            "形容詞,接尾,*,*,形容詞・アウオ段,基本形,っぽい,ッポイ,ッポイ",
        );
        assert_eq!(positions(&[HON, ppoi]), (0, 1));
        // Each of the three after a particle: the last one wins.
        assert_eq!(positions(&[HON, WO, reru]), (0, 2));
        assert_eq!(positions(&[HON, WO, nikui]), (0, 2));
        assert_eq!(positions(&[HON, WO, ppoi]), (0, 2));
        // 助詞 and 助動詞 alone.
        assert_eq!(positions(&[HON, DA]), (0, 1));
        assert_eq!(positions(&[HON, WO]), (0, 1));
    }

    #[test]
    fn suru_and_dekiru_are_function_words() {
        // 利用 / し / た
        assert_eq!(positions(&[RIYOU, SHI, TA]), (0, 2));
        assert_eq!(positions(&[RIYOU, SHI]), (0, 1));
        // する at the start of a bunsetsu is the head.
        assert_eq!(positions(&[SHI, TA]), (0, 1));
        assert_eq!(positions(&[SHI]), (0, 0));
        // Another verb after a verbal noun is the head.
        assert_eq!(positions(&[RIYOU, YON, DA]), (1, 2));
        // できる too, in either spelling of its base form.
        let dekiru = ("でき", "動詞,自立,*,*,一段,連用形,できる,デキ,デキ");
        assert_eq!(positions(&[RIYOU, dekiru, TA]), (0, 2));
        let dekiru = ("出来", "動詞,自立,*,*,一段,連用形,出来る,デキ,デキ");
        assert_eq!(positions(&[RIYOU, dekiru, TA]), (0, 2));
    }

    #[test]
    fn a_dependent_noun_and_the_suffix_teki_are_not_heads() {
        // 読む / こと / が
        assert_eq!(positions(&[YOMU, KOTO, GA]), (0, 2));
        // At the start of a bunsetsu it stays the head, as nothing else is.
        assert_eq!(positions(&[KOTO, GA]), (0, 1));
        // 基本 / 的 / に
        assert_eq!(positions(&[KIHON, TEKI, NI]), (0, 2));
        // 長 / さ / は: the suffix さ (名詞,接尾,特殊).
        let naga = (
            "長",
            "形容詞,自立,*,*,形容詞・アウオ段,ガル接続,長い,ナガ,ナガ",
        );
        let sa = ("さ", "名詞,接尾,特殊,*,*,*,さ,サ,サ");
        assert_eq!(positions(&[naga, sa, GA]), (0, 2));
        // Another suffix is a head: 本 / 人 (名詞,接尾,助数詞).
        let nin = ("人", "名詞,接尾,助数詞,*,*,*,人,ニン,ニン");
        assert_eq!(positions(&[HON, nin, GA]), (1, 2));
    }

    #[test]
    fn a_verb_after_a_particle_is_part_of_a_compound_function_word() {
        let naru = ("なっ", "動詞,自立,*,*,五段・ラ行,連用タ接続,なる,ナッ,ナッ");
        // 読む / こと / に / なっ / た
        assert_eq!(positions(&[YOMU, KOTO, NI, naru, TA]), (0, 4));
        // 本 / に / なっ / た: the same in a shorter bunsetsu.
        assert_eq!(positions(&[HON, NI, naru, TA]), (0, 3));
        // A verb after a noun is the head.
        assert_eq!(positions(&[HON, naru, TA]), (1, 2));
        // At the start of a bunsetsu too.
        assert_eq!(positions(&[naru, TA]), (0, 1));
    }

    #[test]
    fn no_morpheme_has_no_positions() {
        let none: [LatticeMorpheme; 0] = [];
        assert_eq!(Schema::ipadic().positions(&none), None);
    }

    #[test]
    fn morphemes_with_fewer_columns_do_not_panic() {
        // No column, one column, and the columns up to the part of speech.
        assert_eq!(positions(&[("x", ""), ("y", "")]), (1, 1));
        assert_eq!(positions(&[("本", "名詞"), ("を", "助詞")]), (0, 1));
        assert_eq!(
            positions(&[("利用", "名詞,サ変接続"), ("し", "動詞,自立")]),
            (1, 1)
        );
        let schema = Schema::ipadic();
        let bare = LatticeMorpheme::new("x", "");
        assert!(!schema.is_symbol(&bare));
        assert_eq!(schema.part_of_speech_of(&bare), "");
        // A symbol by its surface needs no column.
        assert!(schema.is_comma(&LatticeMorpheme::new(",", "")));
    }

    #[test]
    fn a_schema_without_patterns_takes_the_last_morpheme() {
        // Every morpheme is a head, none a function word.
        let schema = Schema {
            part_of_speech: 0..4,
            ..Schema::default()
        };
        assert_eq!(schema.positions(&morphemes(&[HON, WO, MARU])), Some((2, 2)));
    }

    #[test]
    fn punctuation_and_brackets_are_recognized_by_part_of_speech_and_by_surface() {
        let schema = Schema::ipadic();
        let unknown = |surface: &str| LatticeMorpheme::new(surface, "名詞,サ変接続,*,*,*,*,*,*,*");
        let known = |pair: (&str, &str)| LatticeMorpheme::new(pair.0, pair.1);

        assert!(schema.is_comma(&known(TEN)));
        assert!(schema.is_period(&known(MARU)));
        assert!(schema.is_open_bracket(&known(KAGI)));
        assert!(schema.is_close_bracket(&known(KAGI_TOJI)));
        for surface in [",", "，", "、"] {
            assert!(schema.is_comma(&unknown(surface)), "{surface}");
            assert!(!schema.is_period(&unknown(surface)), "{surface}");
        }
        for surface in [".", "．", "。"] {
            assert!(schema.is_period(&unknown(surface)), "{surface}");
            assert!(!schema.is_comma(&unknown(surface)), "{surface}");
        }
        for surface in ["(", "（", "[", "「", "『", "“"] {
            assert!(schema.is_open_bracket(&unknown(surface)), "{surface}");
            assert!(!schema.is_close_bracket(&unknown(surface)), "{surface}");
        }
        for surface in [")", "）", "]", "」", "』", "”"] {
            assert!(schema.is_close_bracket(&unknown(surface)), "{surface}");
            assert!(!schema.is_open_bracket(&unknown(surface)), "{surface}");
        }
        // Each of them is a symbol, and so are the other marks.
        for surface in [
            ",", "。", "(", "」", "!", "?", "-", "/", "・", "…", ",“", "!?",
        ] {
            assert!(schema.is_symbol(&unknown(surface)), "{surface}");
        }
        // By the part of speech alone: the half-width forms of 、 。 「 」
        // are in no list of characters.
        let comma = LatticeMorpheme::new("､", "記号,読点,*,*,*,*,､,､,､");
        let period = LatticeMorpheme::new("｡", "記号,句点,*,*,*,*,｡,｡,｡");
        let open = LatticeMorpheme::new("｢", "記号,括弧開,*,*,*,*,｢,｢,｢");
        let close = LatticeMorpheme::new("｣", "記号,括弧閉,*,*,*,*,｣,｣,｣");
        assert!(schema.is_comma(&comma) && !schema.is_period(&comma));
        assert!(schema.is_period(&period) && !schema.is_comma(&period));
        assert!(schema.is_open_bracket(&open) && !schema.is_close_bracket(&open));
        assert!(schema.is_close_bracket(&close) && !schema.is_open_bracket(&close));
        assert!(!schema.is_comma(&unknown("､")));
        // The part of speech 記号 alone is a symbol of no finer kind.
        let alphabet = LatticeMorpheme::new("Ａ", "記号,アルファベット,*,*,*,*,Ａ,エイ,エイ");
        assert!(schema.is_symbol(&alphabet));
        assert!(!schema.is_comma(&alphabet) && !schema.is_period(&alphabet));
        // A unit, a word and a number are not symbols.
        for surface in ["%", "°", "%、", "本", "1", "a"] {
            assert!(!schema.is_symbol(&unknown(surface)), "{surface}");
        }
        assert!(!schema.is_comma(&known(HON)));
    }

    #[test]
    fn the_brackets_come_in_pairs() {
        assert_eq!(
            OPEN_BRACKETS.chars().count(),
            CLOSE_BRACKETS.chars().count()
        );
        // No character is of two kinds.
        let all = [
            COMMAS,
            PERIODS,
            OPEN_BRACKETS,
            CLOSE_BRACKETS,
            OTHER_SYMBOLS,
        ]
        .concat();
        let mut characters: Vec<char> = all.chars().collect();
        let count = characters.len();
        characters.sort_unstable();
        characters.dedup();
        assert_eq!(characters.len(), count);
    }

    #[test]
    fn the_part_of_speech_is_joined_up_to_the_first_empty_level() {
        let schema = Schema::ipadic();
        let of =
            |pair: (&str, &str)| schema.part_of_speech_of(&LatticeMorpheme::new(pair.0, pair.1));
        assert_eq!(of(HON), "名詞-一般");
        assert_eq!(of(TEKI), "名詞-接尾-形容動詞語幹");
        assert_eq!(of(DA), "助動詞");
        assert_eq!(of(("x", "*,名詞")), "");
    }

    #[test]
    fn particles_and_words_taken_by_their_surface_follow_the_patterns() {
        let schema = Schema::ipadic();
        let bunsetsu = morphemes(&[
            (
                "ゆっくり",
                "副詞,助詞類接続,*,*,*,*,ゆっくり,ユックリ,ユックリ",
            ),
            ("と", "助詞,格助詞,一般,*,*,*,と,ト,ト"),
            ("この", "連体詞,*,*,*,*,*,この,コノ,コノ"),
            ("しかし", "接続詞,*,*,*,*,*,しかし,シカシ,シカシ"),
            ("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
        ]);
        // The last index is past the bunsetsu.
        let particles: Vec<bool> = (0..6)
            .map(|index| schema.is_particle(&bunsetsu, index))
            .collect();
        assert_eq!(particles, [false, true, false, false, false, false]);
        let lexicalized: Vec<bool> = (0..6)
            .map(|index| schema.is_lexicalized(&bunsetsu, index))
            .collect();
        assert_eq!(lexicalized, [true, true, true, true, false, false]);
    }

    #[test]
    fn unidic_is_a_second_schema_of_data_only() {
        let schema = Schema::unidic();
        // 思っ / て / いる: いる (非自立可能) is a function word.
        let omotte_iru = morphemes(&[
            (
                "思っ",
                "動詞,一般,*,*,五段-ワア行,連用形-促音便,オモウ,思う",
            ),
            ("て", "助詞,接続助詞,*,*,*,*,テ,て"),
            (
                "いる",
                "動詞,非自立可能,*,*,上一段-ア行,終止形-一般,イル,居る",
            ),
        ]);
        assert_eq!(schema.positions(&omotte_iru), Some((0, 2)));

        // 事故 / に / よっ / て: よっ after に is not the head.
        let ni_yotte = morphemes(&[
            ("事故", "名詞,普通名詞,一般,*,*,*,ジコ,事故"),
            ("に", "助詞,格助詞,*,*,*,*,ニ,に"),
            ("よっ", "動詞,一般,*,*,五段-ラ行,連用形-促音便,ヨル,因る"),
            ("て", "助詞,接続助詞,*,*,*,*,テ,て"),
        ]);
        assert_eq!(schema.positions(&ni_yotte), Some((0, 3)));

        // 本 / が / 読む is not such a verb: が is not に, と or を.
        let ga_yomu = morphemes(&[
            ("本", "名詞,普通名詞,一般,*,*,*,ホン,本"),
            ("が", "助詞,格助詞,*,*,*,*,ガ,が"),
            ("読む", "動詞,一般,*,*,五段-マ行,終止形-一般,ヨム,読む"),
        ]);
        assert_eq!(schema.positions(&ga_yomu), Some((2, 2)));

        // 。 is 補助記号,句点.
        let period = LatticeMorpheme::new("。", "補助記号,句点,*,*,*,*,,。");
        assert!(schema.is_period(&period) && schema.is_symbol(&period));
        // The schema of IPADIC does not know these parts of speech.
        assert_eq!(Schema::ipadic().positions(&omotte_iru), Some((2, 2)));
    }

    const SURU_UNIDIC: (&str, &str) =
        ("する", "動詞,非自立可能,*,*,サ行変格,連体形-一般,スル,為る");
    const TA_UNIDIC: (&str, &str) = ("た", "助動詞,*,*,*,助動詞-タ,連体形-一般,タ,た");
    const NI_UNIDIC: (&str, &str) = ("に", "助詞,格助詞,*,*,*,*,ニ,に");
    const TSUKAU_UNIDIC: (&str, &str) =
        ("使う", "動詞,一般,*,*,五段-ワア行,連体形-一般,ツカウ,使う");

    #[test]
    fn unidic_does_not_take_an_auxiliary_stem_as_the_head() {
        let schema = Schema::unidic();
        // 使う / よう / に: よう is 形状詞,助動詞語幹.
        let you = morphemes(&[
            TSUKAU_UNIDIC,
            ("よう", "形状詞,助動詞語幹,*,*,*,*,ヨウ,様"),
            ("に", "助動詞,*,*,*,助動詞-ダ,連用形-ニ,ダ,だ"),
        ]);
        assert_eq!(schema.positions(&you), Some((0, 2)));
        // 働い / た / そう / だ: そう of hearsay is 名詞,助動詞語幹.
        let sou = morphemes(&[
            (
                "働い",
                "動詞,一般,*,*,五段-カ行,連用形-イ音便,ハタラク,働く",
            ),
            TA_UNIDIC,
            ("そう", "名詞,助動詞語幹,*,*,*,*,ソウ,そう-伝聞"),
            ("だ", "助動詞,*,*,*,助動詞-ダ,終止形-一般,ダ,だ"),
        ]);
        assert_eq!(schema.positions(&sou), Some((0, 3)));
    }

    #[test]
    fn unidic_does_not_take_a_formal_noun_after_a_predicate_as_the_head() {
        let schema = Schema::unidic();
        let koto = ("こと", "名詞,普通名詞,一般,*,*,*,コト,事");
        let tame = ("ため", "名詞,普通名詞,副詞可能,*,*,*,タメ,為");
        let kounyuu = ("購入", "名詞,普通名詞,サ変可能,*,*,*,コウニュウ,購入");
        // 購入 / する / こと / に: after a verb.
        let after_verb = morphemes(&[kounyuu, SURU_UNIDIC, koto, NI_UNIDIC]);
        assert_eq!(schema.positions(&after_verb), Some((0, 3)));
        // 購入 / する / た / ため / に: after an auxiliary.
        let after_auxiliary = morphemes(&[kounyuu, SURU_UNIDIC, TA_UNIDIC, tame, NI_UNIDIC]);
        assert_eq!(schema.positions(&after_auxiliary), Some((0, 4)));
        // 新しい / こと / に: after an adjective.
        let after_adjective = morphemes(&[
            (
                "新しい",
                "形容詞,一般,*,*,形容詞,連体形-一般,アタラシイ,新しい",
            ),
            koto,
            NI_UNIDIC,
        ]);
        assert_eq!(schema.positions(&after_adjective), Some((0, 2)));
        // こと / に, and 購入 / こと / に: not after one of them.
        assert_eq!(
            schema.positions(&morphemes(&[koto, NI_UNIDIC])),
            Some((0, 1))
        );
        assert_eq!(
            schema.positions(&morphemes(&[kounyuu, koto, NI_UNIDIC])),
            Some((1, 2))
        );
        // 使う / 人 / に: 人 is not a formal noun.
        let hito = morphemes(&[
            TSUKAU_UNIDIC,
            ("人", "名詞,普通名詞,一般,*,*,*,ヒト,人"),
            NI_UNIDIC,
        ]);
        assert_eq!(schema.positions(&hito), Some((1, 2)));
    }

    #[test]
    fn unidic_does_not_take_a_nominal_suffix_after_an_adjective_as_the_head() {
        let schema = Schema::unidic();
        // 大き / さ / に.
        let ookisa = morphemes(&[
            ("大き", "形容詞,一般,*,*,形容詞,語幹-一般,オオキイ,大きい"),
            ("さ", "接尾辞,名詞的,一般,*,*,*,サ,さ"),
            NI_UNIDIC,
        ]);
        assert_eq!(schema.positions(&ookisa), Some((0, 2)));
        // 市 / 長 / に: after a noun the suffix is the head.
        let shichou = morphemes(&[
            ("市", "名詞,普通名詞,一般,*,*,*,シ,市"),
            ("長", "接尾辞,名詞的,一般,*,*,*,チョウ,長"),
            NI_UNIDIC,
        ]);
        assert_eq!(schema.positions(&shichou), Some((1, 2)));
    }

    #[test]
    fn unidic_takes_a_letter_as_a_symbol() {
        let schema = Schema::unidic();
        // 5 / k / m: the letters are 記号,文字.
        let kilometers = morphemes(&[
            ("5", "名詞,数詞,*,*,*,*,ゴ,五"),
            ("k", "記号,文字,*,*,*,*,ケー,Ｋ"),
            ("m", "記号,文字,*,*,*,*,エム,Ｍ"),
        ]);
        assert!(schema.is_symbol(&kilometers[1]));
        assert_eq!(schema.positions(&kilometers), Some((0, 0)));
    }

    #[test]
    fn a_schema_is_chosen_by_the_name_of_the_dictionary() {
        assert_eq!(Schema::for_dictionary("ipadic"), Some(Schema::ipadic()));
        for name in ["unidic", "unidic-cwj", "unidic-csj"] {
            assert_eq!(Schema::for_dictionary(name), Some(Schema::unidic()));
        }
        // Every name of the list has a schema, and no other name has.
        for name in Schema::DICTIONARIES {
            assert!(Schema::for_dictionary(name).is_some(), "{name}");
        }
        for name in ["", "IPADIC", "ipadic-neologd", "ko-dic", "unidic-"] {
            assert_eq!(Schema::for_dictionary(name), None, "{name}");
        }
    }

    #[test]
    fn a_schema_comes_back_from_its_bytes() {
        for schema in [Schema::ipadic(), Schema::unidic(), Schema::default()] {
            let mut bytes = Vec::new();
            schema.write(&mut bytes);
            let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
            assert_eq!(Schema::read(&mut reader).unwrap(), schema);
            assert_eq!(reader.remaining(), 0);
        }
    }

    #[test]
    fn the_two_lists_of_patterns_keep_their_places_in_the_bytes() {
        let schema = Schema {
            function_words: vec![Pattern::new(&["助詞"])],
            not_heads: vec![Pattern::new(&["名詞", "非自立"]), Pattern::default()],
            ..Schema::default()
        };
        let mut bytes = Vec::new();
        schema.write(&mut bytes);
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        let read = Schema::read(&mut reader).unwrap();
        assert_eq!(read.function_words, [Pattern::new(&["助詞"])]);
        assert_eq!(read.not_heads.len(), 2);
    }

    #[test]
    fn bytes_that_end_inside_a_schema_are_an_error() {
        let mut bytes = Vec::new();
        Schema::ipadic().write(&mut bytes);
        for length in [0, 4, 20, 24, bytes.len() / 2, bytes.len() - 1] {
            let mut reader = Reader::new(&bytes[..length], "test bytes", Error::Chunker);
            let message = Schema::read(&mut reader).unwrap_err().to_string();
            assert!(message.contains("ends early"), "{length}: {message}");
        }
    }

    #[test]
    fn the_width_is_the_number_of_columns_the_schema_reads() {
        // IPADIC: four levels of part of speech, the conjugation type and
        // form, the base form.
        assert_eq!(Schema::ipadic().width(), 7);
        let mut schema = Schema::ipadic();
        schema.base_form = None;
        assert_eq!(schema.width(), 6);
        schema.conjugation_form = None;
        schema.conjugation_type = Some(9);
        assert_eq!(schema.width(), 10);
        schema.conjugation_type = None;
        assert_eq!(schema.width(), 4);
        schema.part_of_speech = 0..0;
        assert_eq!(schema.width(), 0);
    }

    #[test]
    fn morphemes_with_fewer_columns_than_the_schema_reads_are_refused() {
        let schema = Schema::ipadic();
        let wide = LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ");
        // An unknown word of MeCab has no reading: seven columns.
        let unknown = LatticeMorpheme::new("x", "名詞,一般,*,*,*,*,*");
        let narrow = LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*");
        let none: [LatticeMorpheme; 0] = [];
        assert!(schema.check(&none).is_ok());
        assert!(schema.check(&[wide.clone(), unknown.clone()]).is_ok());

        let error = schema.check(&[wide.clone(), unknown, narrow]).unwrap_err();
        assert!(matches!(error, Error::Dictionary(_)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "the morpheme 3 (\"彼\") has fewer than the 7 feature columns the schema reads: it is not of the dictionary of the model"
        );

        // A schema that reads nothing refuses nothing.
        let mut schema = Schema::ipadic();
        schema.part_of_speech = 0..0;
        schema.conjugation_type = None;
        schema.conjugation_form = None;
        schema.base_form = None;
        assert!(schema.check(&[LatticeMorpheme::new("彼", "")]).is_ok());
    }
}

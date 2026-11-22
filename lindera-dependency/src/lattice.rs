//! The lattice format of CaboCha (`cabocha -f1`): reading and writing.
//!
//! ```text
//! * 0 2D 0/1 0.000000
//! 彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
//! は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
//! * 1 2D 0/1 0.000000
//! 本\t名詞,一般,*,*,*,*,本,ホン,ホン
//! を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
//! * 2 -1D 0/1 0.000000
//! 読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
//! だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
//! 。\t記号,句点,*,*,*,*,。,。,。
//! EOS
//! ```
//!
//! (`\t` stands for a tab.)
//!
//! # The lines
//!
//! A line is, in this order:
//!
//! 1. a morpheme, if it holds a tab: the surface, the feature string and,
//!    optionally, a named entity tag, separated by tabs. The tab is what tells
//!    a morpheme from the other lines, so a surface may be `*`, start with
//!    `#`, or be `EOS`;
//! 2. the end of a sentence, if it is `EOS`;
//! 3. a bunsetsu line, if it starts with `*` and a space: the id of the
//!    bunsetsu, counted from 0; the id of its head, `-1` for the root,
//!    followed by the letter of the arc type (see
//!    [`ArcType`]; `D` when there is none); then, optionally,
//!    the positions of the head word and of the function word in the
//!    bunsetsu, as `head/function`, and the score;
//! 4. a comment, if it starts with `#`. Comments stand before the first line
//!    of a sentence, and belong to it.
//!
//! Anything else, an empty line included, is an error, as is an input that
//! ends inside a sentence. Lines end with a line feed.
//!
//! # Reading and writing back
//!
//! A sentence without bunsetsu lines is a sentence of morphemes only. A
//! bunsetsu line with the head only is what CaboCha writes for its chunk
//! layer, `* 0 2D` followed by a space, and is written back so. The writer
//! gives back the bytes the reader read, with two exceptions: a score is
//! written with six decimals, as CaboCha writes it, and a bunsetsu line with
//! the head only gets the space after it.
//!
//! The reader checks the structure of a sentence ([`Sentence::check`]), not
//! its tree: a sentence with two roots or with crossing arcs is read, and
//! [`Sentence::violations`] reports it.

use std::borrow::Cow;
use std::io::{BufRead, Write};

use crate::error::{Error, Result};
use crate::morpheme::{Morpheme, split_features};
use crate::sentence::{ArcType, Bunsetsu, Sentence};

/// The line that ends a sentence.
const END_OF_SENTENCE: &str = "EOS";

/// A morpheme of the lattice format.
///
/// It keeps the feature string as it is in the input, next to the columns it
/// is split into, so that writing it gives the same bytes whether a column
/// with a comma was quoted (MeCab) or not (`lindera tokenize -o mecab`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatticeMorpheme {
    surface: String,
    features: String,
    columns: Vec<String>,
    named_entity: Option<String>,
}

impl LatticeMorpheme {
    /// Makes a morpheme from a surface and a feature string.
    ///
    /// # Arguments
    ///
    /// * `surface` - The surface.
    /// * `features` - The feature string: the columns of the dictionary,
    ///   separated by commas, a column with a comma quoted as in CSV.
    ///
    /// # Returns
    ///
    /// The morpheme, without a named entity tag.
    pub fn new(surface: impl Into<String>, features: impl Into<String>) -> Self {
        let features = features.into();
        let columns = split_features(&features);
        Self {
            surface: surface.into(),
            features,
            columns,
            named_entity: None,
        }
    }

    /// Sets the named entity tag of the morpheme.
    ///
    /// # Arguments
    ///
    /// * `named_entity` - The tag, or `None` for no tag.
    ///
    /// # Returns
    ///
    /// The morpheme with the tag.
    pub fn with_named_entity(mut self, named_entity: Option<String>) -> Self {
        self.named_entity = named_entity;
        self
    }
}

impl Morpheme for LatticeMorpheme {
    fn surface(&self) -> &str {
        &self.surface
    }

    fn feature(&self, index: usize) -> Option<&str> {
        self.columns.get(index).map(String::as_str)
    }

    fn features(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.features)
    }

    fn named_entity(&self) -> Option<&str> {
        self.named_entity.as_deref()
    }
}

/// Reads every sentence of a text in the lattice format.
///
/// # Arguments
///
/// * `input` - The text.
///
/// # Returns
///
/// The sentences, or the first error; see [`Reader`].
///
/// # Examples
///
/// ```
/// use lindera_dependency::lattice;
///
/// let text = "* 0 1D 0/1 0.000000\n\
///     本\t名詞,一般,*,*,*,*,本,ホン,ホン\n\
///     を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n\
///     * 1 -1D 0/0 0.000000\n\
///     読む\t動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム\n\
///     EOS\n";
/// let sentences = lattice::parse(text)?;
/// assert_eq!(sentences.len(), 1);
/// assert_eq!(sentences[0].bunsetsu[0].head, Some(1));
/// # Ok::<(), lindera_dependency::Error>(())
/// ```
pub fn parse(input: &str) -> Result<Vec<Sentence<LatticeMorpheme>>> {
    Reader::new(input.as_bytes()).collect()
}

/// Reads the sentences of an input in the lattice format, one at a time.
///
/// An iterator of `Result<Sentence<LatticeMorpheme>>`. An error is
/// [`Error::Format`] with the number of the line that is wrong, or
/// [`Error::Io`]; after an error, the iterator ends.
#[derive(Debug)]
pub struct Reader<R> {
    input: R,
    /// The number of the last line read, from 1.
    line: usize,
    /// The buffer a line is read into.
    buffer: String,
    /// Set after an error or the end of the input.
    done: bool,
}

impl<R: BufRead> Reader<R> {
    /// Makes a reader of an input.
    ///
    /// # Arguments
    ///
    /// * `input` - The input, in UTF-8.
    ///
    /// # Returns
    ///
    /// The reader.
    pub fn new(input: R) -> Self {
        Self {
            input,
            line: 0,
            buffer: String::new(),
            done: false,
        }
    }

    /// Reads the next line into the buffer, without its line feed.
    ///
    /// # Returns
    ///
    /// `false` at the end of the input.
    fn read_line(&mut self) -> Result<bool> {
        self.buffer.clear();
        if self.input.read_line(&mut self.buffer)? == 0 {
            return Ok(false);
        }
        self.line += 1;
        if self.buffer.ends_with('\n') {
            self.buffer.pop();
        }
        Ok(true)
    }

    /// Reads the next sentence.
    ///
    /// # Returns
    ///
    /// The sentence, or `None` at the end of the input.
    fn read_sentence(&mut self) -> Result<Option<Sentence<LatticeMorpheme>>> {
        let mut sentence = Sentence {
            comments: Vec::new(),
            morphemes: Vec::new(),
            bunsetsu: Vec::new(),
        };
        // The line of each bunsetsu, for the errors found at the end of the
        // sentence.
        let mut bunsetsu_lines = Vec::new();
        // Whether a morpheme has been read before the first bunsetsu line.
        let mut morphemes_first = false;

        loop {
            if !self.read_line()? {
                if sentence.comments.is_empty()
                    && sentence.morphemes.is_empty()
                    && sentence.bunsetsu.is_empty()
                {
                    return Ok(None);
                }
                return Err(Error::format(
                    self.line,
                    "the input ends inside a sentence, without `EOS`",
                ));
            }
            let line = self.buffer.as_str();

            if line.contains('\t') {
                let morpheme = parse_morpheme(line).map_err(|m| Error::format(self.line, m))?;
                match sentence.bunsetsu.last_mut() {
                    Some(bunsetsu) => bunsetsu.morphemes.end += 1,
                    None => morphemes_first = true,
                }
                sentence.morphemes.push(morpheme);
            } else if line == END_OF_SENTENCE {
                break;
            } else if line.starts_with("* ") {
                if morphemes_first {
                    return Err(Error::format(
                        self.line,
                        "a bunsetsu line follows morphemes that belong to no bunsetsu",
                    ));
                }
                if let Some(last) = sentence.bunsetsu.last()
                    && last.morphemes.is_empty()
                {
                    return Err(Error::format(
                        self.line,
                        format!("bunsetsu {} has no morpheme", sentence.bunsetsu.len() - 1),
                    ));
                }
                let bunsetsu =
                    parse_bunsetsu(line, sentence.bunsetsu.len(), sentence.morphemes.len())
                        .map_err(|m| Error::format(self.line, m))?;
                sentence.bunsetsu.push(bunsetsu);
                bunsetsu_lines.push(self.line);
            } else if line.starts_with('#') {
                if !sentence.morphemes.is_empty() || !sentence.bunsetsu.is_empty() {
                    return Err(Error::format(self.line, "a comment inside a sentence"));
                }
                sentence.comments.push(line.to_string());
            } else if line.is_empty() {
                return Err(Error::format(self.line, "an empty line"));
            } else {
                return Err(Error::format(
                    self.line,
                    format!("not a morpheme, a bunsetsu line, a comment or `EOS`: {line:?}"),
                ));
            }
        }

        // What can only be told once the sentence is complete.
        let count = sentence.bunsetsu.len();
        for (index, bunsetsu) in sentence.bunsetsu.iter().enumerate() {
            let line = bunsetsu_lines[index];
            let size = bunsetsu.morphemes.len();
            if size == 0 {
                return Err(Error::format(
                    line,
                    format!("bunsetsu {index} has no morpheme"),
                ));
            }
            if let Some(head) = bunsetsu.head
                && head >= count
            {
                return Err(Error::format(
                    line,
                    format!(
                        "the head of bunsetsu {index} is {head}, but there are {count} bunsetsu"
                    ),
                ));
            }
            if let Some((head_word, function_word)) = bunsetsu.positions
                && (head_word >= size || function_word >= size)
            {
                return Err(Error::format(
                    line,
                    format!(
                        "the positions {head_word}/{function_word} of bunsetsu {index} are outside its {size} morpheme(s)"
                    ),
                ));
            }
        }

        Ok(Some(sentence))
    }
}

impl<R: BufRead> Iterator for Reader<R> {
    type Item = Result<Sentence<LatticeMorpheme>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.read_sentence() {
            Ok(Some(sentence)) => Some(Ok(sentence)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(error))
            }
        }
    }
}

/// Reads a morpheme line.
///
/// # Arguments
///
/// * `line` - The line, which holds a tab.
///
/// # Returns
///
/// The morpheme, or what is wrong with the line.
fn parse_morpheme(line: &str) -> std::result::Result<LatticeMorpheme, String> {
    let mut columns = line.split('\t');
    let surface = columns.next().unwrap_or_default();
    let features = columns.next().unwrap_or_default();
    let named_entity = columns.next();
    if columns.next().is_some() {
        return Err("a morpheme line has more than three columns".to_string());
    }
    if surface.is_empty() {
        return Err("a morpheme line has no surface".to_string());
    }
    if features.is_empty() {
        return Err("a morpheme line has no feature".to_string());
    }
    Ok(LatticeMorpheme::new(surface, features).with_named_entity(named_entity.map(str::to_string)))
}

/// Reads a bunsetsu line.
///
/// # Arguments
///
/// * `line` - The line, which starts with `*` and a space.
/// * `index` - The index the bunsetsu has to have.
/// * `start` - The index of its first morpheme.
///
/// # Returns
///
/// The bunsetsu, without a morpheme yet, or what is wrong with the line.
fn parse_bunsetsu(line: &str, index: usize, start: usize) -> std::result::Result<Bunsetsu, String> {
    let mut columns: Vec<&str> = line[2..].split(' ').collect();
    // CaboCha ends the line of its chunk layer, `* 0 2D `, with a space.
    if columns.len() == 3 && columns[2].is_empty() {
        columns.pop();
    }
    if !(2..=4).contains(&columns.len()) || columns.iter().any(|column| column.is_empty()) {
        return Err(format!(
            "a bunsetsu line is `* id head`, `* id head head/function` or `* id head head/function score`: {line:?}"
        ));
    }

    let id: usize = columns[0]
        .parse()
        .map_err(|_| format!("the id of a bunsetsu is not a number: {:?}", columns[0]))?;
    if id != index {
        return Err(format!("the id of the bunsetsu is {id}, not {index}"));
    }

    let (head, arc_type) = parse_head(columns[1])?;

    let positions = match columns.get(2) {
        Some(column) => {
            let parsed = column
                .split_once('/')
                .and_then(|(head_word, function_word)| {
                    Some((head_word.parse().ok()?, function_word.parse().ok()?))
                });
            Some(parsed.ok_or_else(|| {
                format!("the positions of a bunsetsu are not `head/function`: {column:?}")
            })?)
        }
        None => None,
    };

    let score = match columns.get(3) {
        Some(column) => {
            let score: f64 = column
                .parse()
                .map_err(|_| format!("the score of a bunsetsu is not a number: {column:?}"))?;
            if !score.is_finite() {
                return Err(format!("the score of a bunsetsu is not finite: {column:?}"));
            }
            Some(score)
        }
        None => None,
    };

    Ok(Bunsetsu {
        morphemes: start..start,
        head,
        arc_type,
        positions,
        score,
    })
}

/// Reads the head column of a bunsetsu line: the id of the head, `-1` for the
/// root, and the letter of the arc type.
///
/// # Arguments
///
/// * `column` - The column, such as `2D`, `-1D` or `2`.
///
/// # Returns
///
/// The head and the arc type (`D` without a letter), or what is wrong.
fn parse_head(column: &str) -> std::result::Result<(Option<usize>, ArcType), String> {
    let (number, arc_type) = match column.chars().last() {
        Some(letter) if letter.is_ascii_alphabetic() => {
            let arc_type = ArcType::from_char(letter)
                .ok_or_else(|| format!("unknown arc type {letter:?} in {column:?}"))?;
            (&column[..column.len() - 1], arc_type)
        }
        _ => (column, ArcType::Dependency),
    };
    if number == "-1" {
        return Ok((None, arc_type));
    }
    let head = number
        .parse()
        .map_err(|_| format!("the head of a bunsetsu is not `-1` or the id of one: {column:?}"))?;
    Ok((Some(head), arc_type))
}

/// Writes a sentence in the lattice format.
///
/// The comments, then each bunsetsu line followed by its morphemes (the
/// morphemes alone for a sentence without bunsetsu), then `EOS`.
///
/// # Arguments
///
/// * `writer` - Where to write.
/// * `sentence` - The sentence.
///
/// # Returns
///
/// `Ok(())`; [`Error::Structure`], before anything is written, when
/// [`Sentence::check`] fails or a morpheme cannot stand on a line (an empty
/// surface or feature string, a tab or a line break in one of its strings);
/// or [`Error::Io`].
///
/// # Examples
///
/// ```
/// use lindera_dependency::lattice::{self, LatticeMorpheme};
/// use lindera_dependency::{Bunsetsu, Sentence};
///
/// let sentence = Sentence::new(
///     vec![
///         LatticeMorpheme::new("本", "名詞,一般"),
///         LatticeMorpheme::new("を", "助詞,格助詞"),
///         LatticeMorpheme::new("読む", "動詞,自立"),
///     ],
///     vec![Bunsetsu::new(0..2, Some(1)), Bunsetsu::new(2..3, None)],
/// )?;
/// let mut output = Vec::new();
/// lattice::write(&mut output, &sentence)?;
/// assert_eq!(
///     String::from_utf8(output).unwrap(),
///     "* 0 1D \n本\t名詞,一般\nを\t助詞,格助詞\n* 1 -1D \n読む\t動詞,自立\nEOS\n"
/// );
/// # Ok::<(), lindera_dependency::Error>(())
/// ```
pub fn write<W: Write, M: Morpheme>(writer: &mut W, sentence: &Sentence<M>) -> Result<()> {
    sentence.check()?;
    for (index, morpheme) in sentence.morphemes.iter().enumerate() {
        check_morpheme(index, morpheme)?;
    }

    for comment in &sentence.comments {
        writeln!(writer, "{comment}")?;
    }
    if sentence.bunsetsu.is_empty() {
        for morpheme in &sentence.morphemes {
            write_morpheme(writer, morpheme)?;
        }
    }
    for (index, bunsetsu) in sentence.bunsetsu.iter().enumerate() {
        write!(writer, "* {index} ")?;
        match bunsetsu.head {
            Some(head) => write!(writer, "{head}")?,
            None => write!(writer, "-1")?,
        }
        write!(writer, "{}", bunsetsu.arc_type.as_char())?;
        match (bunsetsu.positions, bunsetsu.score) {
            (Some((head_word, function_word)), Some(score)) => {
                writeln!(writer, " {head_word}/{function_word} {score:.6}")?
            }
            (Some((head_word, function_word)), None) => {
                writeln!(writer, " {head_word}/{function_word}")?
            }
            // As CaboCha writes its chunk layer.
            (None, _) => writeln!(writer, " ")?,
        }
        for morpheme in &sentence.morphemes[bunsetsu.morphemes.clone()] {
            write_morpheme(writer, morpheme)?;
        }
    }
    writeln!(writer, "{END_OF_SENTENCE}")?;
    Ok(())
}

/// Checks that a morpheme can stand on a line of the lattice format.
///
/// # Arguments
///
/// * `index` - The index of the morpheme in its sentence, for the message.
/// * `morpheme` - The morpheme.
///
/// # Returns
///
/// `Ok(())`, or [`Error::Structure`].
fn check_morpheme<M: Morpheme>(index: usize, morpheme: &M) -> Result<()> {
    let features = morpheme.features();
    if morpheme.surface().is_empty() {
        return Err(Error::Structure(format!("morpheme {index} has no surface")));
    }
    if features.is_empty() {
        return Err(Error::Structure(format!("morpheme {index} has no feature")));
    }
    let strings = [
        Some(morpheme.surface()),
        Some(features.as_ref()),
        morpheme.named_entity(),
    ];
    if strings.iter().flatten().any(|s| s.contains(['\t', '\n'])) {
        return Err(Error::Structure(format!(
            "morpheme {index} holds a tab or a line break"
        )));
    }
    Ok(())
}

/// Writes a morpheme line.
///
/// # Arguments
///
/// * `writer` - Where to write.
/// * `morpheme` - The morpheme.
///
/// # Returns
///
/// `Ok(())`, or the error of the write.
fn write_morpheme<W: Write, M: Morpheme>(writer: &mut W, morpheme: &M) -> Result<()> {
    write!(writer, "{}\t{}", morpheme.surface(), morpheme.features())?;
    if let Some(named_entity) = morpheme.named_entity() {
        write!(writer, "\t{named_entity}")?;
    }
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "* 0 2D 0/1 0.000000\n\
        彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ\n\
        は\t助詞,係助詞,*,*,*,*,は,ハ,ワ\n\
        * 1 2D 0/1 1.234500\n\
        本\t名詞,一般,*,*,*,*,本,ホン,ホン\n\
        を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ\n\
        * 2 -1D 0/1 0.000000\n\
        読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン\n\
        だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ\n\
        。\t記号,句点,*,*,*,*,。,。,。\n\
        EOS\n";

    fn write_all<M: Morpheme>(sentences: &[Sentence<M>]) -> String {
        let mut output = Vec::new();
        for sentence in sentences {
            write(&mut output, sentence).unwrap();
        }
        String::from_utf8(output).unwrap()
    }

    fn round_trip(text: &str) {
        let sentences = parse(text).unwrap();
        assert_eq!(write_all(&sentences), text);
    }

    fn format_error(text: &str) -> (usize, String) {
        match parse(text) {
            Err(Error::Format { line, message }) => (line, message),
            other => panic!("expected a format error, got {other:?}"),
        }
    }

    #[test]
    fn reads_bunsetsu_with_positions_and_scores() {
        let sentences = parse(FULL).unwrap();
        assert_eq!(sentences.len(), 1);
        let sentence = &sentences[0];
        assert_eq!(sentence.morphemes.len(), 7);
        assert_eq!(sentence.bunsetsu.len(), 3);
        assert_eq!(sentence.bunsetsu[0].morphemes, 0..2);
        assert_eq!(sentence.bunsetsu[1].morphemes, 2..4);
        assert_eq!(sentence.bunsetsu[2].morphemes, 4..7);
        assert_eq!(sentence.bunsetsu[0].head, Some(2));
        assert_eq!(sentence.bunsetsu[2].head, None);
        assert_eq!(sentence.bunsetsu[1].positions, Some((0, 1)));
        assert_eq!(sentence.bunsetsu[1].score, Some(1.2345));
        assert_eq!(sentence.morphemes[4].surface(), "読ん");
        assert_eq!(sentence.morphemes[4].feature(6), Some("読む"));
        assert_eq!(sentence.morphemes[4].feature(9), None);
        assert!(sentence.violations().is_empty());
        round_trip(FULL);
    }

    #[test]
    fn keeps_the_head_only_form_of_the_chunk_layer() {
        let text = "* 0 1D \n本\t名詞\nを\t助詞\n* 1 -1D \n読む\t動詞\nEOS\n";
        let sentences = parse(text).unwrap();
        assert_eq!(sentences[0].bunsetsu[0].positions, None);
        assert_eq!(sentences[0].bunsetsu[0].score, None);
        round_trip(text);
    }

    #[test]
    fn reads_a_head_only_line_without_the_trailing_space() {
        let sentences = parse("* 0 -1D\n本\t名詞\nEOS\n").unwrap();
        assert_eq!(write_all(&sentences), "* 0 -1D \n本\t名詞\nEOS\n");
    }

    #[test]
    fn keeps_positions_without_a_score() {
        round_trip("* 0 -1D 0/1\n本\t名詞\nを\t助詞\nEOS\n");
    }

    #[test]
    fn keeps_morphemes_without_bunsetsu() {
        let text = "本\t名詞,一般\nを\t助詞,格助詞\nEOS\n";
        let sentences = parse(text).unwrap();
        assert!(sentences[0].bunsetsu.is_empty());
        assert_eq!(sentences[0].morphemes.len(), 2);
        round_trip(text);
    }

    #[test]
    fn keeps_several_sentences_and_empty_ones() {
        let text = "本\t名詞\nEOS\nEOS\n* 0 -1D 0/0 0.000000\n山\t名詞\nEOS\n";
        let sentences = parse(text).unwrap();
        assert_eq!(sentences.len(), 3);
        assert!(sentences[1].morphemes.is_empty());
        round_trip(text);
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn keeps_comments_before_a_sentence() {
        let text =
            "# S-ID: 1\n#second\n* 0 -1D 0/0 0.000000\n山\t名詞\nEOS\n# S-ID: 2\n川\t名詞\nEOS\n";
        let sentences = parse(text).unwrap();
        assert_eq!(sentences[0].comments, ["# S-ID: 1", "#second"]);
        assert_eq!(sentences[1].comments, ["# S-ID: 2"]);
        round_trip(text);
    }

    #[test]
    fn a_line_with_a_tab_is_a_morpheme_whatever_its_surface() {
        let text = "* 0 1D 0/0 0.000000\n\
            *\t記号,一般,*,*,*,*,*,*,*\n\
            * 1 2D 0/0 0.000000\n\
            #タグ\t名詞,一般,*,*,*,*,*\n\
            * 2 -1D 1/1 0.000000\n\
            * 3\t名詞,一般,*,*,*,*,*\n\
            EOS\t名詞,一般,*,*,*,*,*\n\
            EOS\n";
        let sentences = parse(text).unwrap();
        let sentence = &sentences[0];
        assert_eq!(sentence.bunsetsu.len(), 3);
        let surfaces: Vec<&str> = sentence.morphemes.iter().map(|m| m.surface()).collect();
        assert_eq!(surfaces, ["*", "#タグ", "* 3", "EOS"]);
        assert!(sentence.comments.is_empty());
        round_trip(text);
    }

    #[test]
    fn keeps_the_feature_string_quoted_or_not() {
        let text = ",\t記号,読点,*,*,*,*,\",\",\",\",\",\"\n,\t記号,読点,*,*,*,*,,,,,,\nEOS\n";
        let sentences = parse(text).unwrap();
        let quoted = &sentences[0].morphemes[0];
        assert_eq!(quoted.feature(6), Some(","));
        assert_eq!(quoted.feature(8), Some(","));
        assert_eq!(quoted.feature(9), None);
        let unquoted = &sentences[0].morphemes[1];
        assert_eq!(unquoted.feature(1), Some("読点"));
        assert_eq!(unquoted.feature(6), Some(""));
        assert_eq!(unquoted.feature(11), Some(""));
        round_trip(text);
    }

    #[test]
    fn keeps_the_arc_type() {
        let text = "* 0 1P 0/0 0.000000\n山\t名詞\n* 1 2A 0/0 0.000000\n川\t名詞\n* 2 3I 0/0 0.000000\n海\t名詞\n* 3 -1D 0/0 0.000000\n空\t名詞\nEOS\n";
        let sentences = parse(text).unwrap();
        let arc_types: Vec<ArcType> = sentences[0].bunsetsu.iter().map(|b| b.arc_type).collect();
        assert_eq!(
            arc_types,
            [
                ArcType::Parallel,
                ArcType::Apposition,
                ArcType::Incomplete,
                ArcType::Dependency,
            ]
        );
        round_trip(text);
    }

    #[test]
    fn a_head_without_a_letter_is_a_dependency() {
        let sentences =
            parse("* 0 1 0/0 0.000000\n山\t名詞\n* 1 -1 0/0 0.000000\n川\t名詞\nEOS\n").unwrap();
        assert_eq!(sentences[0].bunsetsu[0].head, Some(1));
        assert_eq!(sentences[0].bunsetsu[0].arc_type, ArcType::Dependency);
        assert_eq!(sentences[0].bunsetsu[1].head, None);
    }

    #[test]
    fn keeps_the_named_entity_tag() {
        let text = "東京\t名詞,固有名詞\tB-LOCATION\nへ\t助詞,格助詞\tO\nEOS\n";
        let sentences = parse(text).unwrap();
        assert_eq!(sentences[0].morphemes[0].named_entity(), Some("B-LOCATION"));
        round_trip(text);
    }

    #[test]
    fn writes_a_score_with_six_decimals() {
        let sentences = parse("* 0 -1D 0/0 1.5\n山\t名詞\nEOS\n").unwrap();
        assert_eq!(
            write_all(&sentences),
            "* 0 -1D 0/0 1.500000\n山\t名詞\nEOS\n"
        );
        round_trip("* 0 -1D 0/0 -12.345678\n山\t名詞\nEOS\n");
    }

    #[test]
    fn reads_a_tree_that_breaks_the_assumptions() {
        let sentences = parse("* 0 -1D \n山\t名詞\n* 1 0D \n川\t名詞\nEOS\n").unwrap();
        assert_eq!(sentences[0].violations().len(), 2);
    }

    #[test]
    fn reads_the_last_line_without_a_line_feed() {
        let sentences = parse("山\t名詞\nEOS").unwrap();
        assert_eq!(sentences.len(), 1);
    }

    #[test]
    fn refuses_ids_out_of_sequence() {
        let (line, message) = format_error("* 0 1D \n山\t名詞\n* 2 -1D \n川\t名詞\nEOS\n");
        assert_eq!(
            (line, message.as_str()),
            (3, "the id of the bunsetsu is 2, not 1")
        );
        let (line, _) = format_error("* 1 -1D \n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
    }

    #[test]
    fn refuses_a_bunsetsu_without_a_morpheme() {
        let (line, message) = format_error("* 0 1D \n* 1 -1D \n川\t名詞\nEOS\n");
        assert_eq!((line, message.as_str()), (2, "bunsetsu 0 has no morpheme"));
        let (line, message) = format_error("* 0 1D \n山\t名詞\n* 1 -1D \nEOS\n");
        assert_eq!((line, message.as_str()), (3, "bunsetsu 1 has no morpheme"));
    }

    #[test]
    fn refuses_a_head_out_of_range() {
        let (line, message) = format_error("* 0 2D \n山\t名詞\n* 1 -1D \n川\t名詞\nEOS\n");
        assert_eq!(
            (line, message.as_str()),
            (1, "the head of bunsetsu 0 is 2, but there are 2 bunsetsu")
        );
        let (line, _) = format_error("* 0 -2D \n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
    }

    #[test]
    fn refuses_positions_out_of_range() {
        let (line, message) = format_error("* 0 -1D 0/1 0.000000\n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
        assert!(message.contains("positions 0/1 of bunsetsu 0"), "{message}");
        let (line, _) = format_error("* 0 -1D 0-1 0.000000\n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
    }

    #[test]
    fn refuses_an_input_that_ends_inside_a_sentence() {
        let (line, message) = format_error("* 0 -1D \n山\t名詞\n");
        assert_eq!(line, 2);
        assert!(message.contains("without `EOS`"), "{message}");
        let (line, _) = format_error("山\t名詞\nEOS\n# comment\n");
        assert_eq!(line, 3);
    }

    #[test]
    fn refuses_an_empty_line() {
        let (line, message) = format_error("山\t名詞\nEOS\n\n川\t名詞\nEOS\n");
        assert_eq!((line, message.as_str()), (3, "an empty line"));
    }

    #[test]
    fn refuses_a_comment_inside_a_sentence() {
        let (line, message) = format_error("山\t名詞\n# comment\nEOS\n");
        assert_eq!((line, message.as_str()), (2, "a comment inside a sentence"));
        let (line, _) = format_error("* 0 -1D \n# comment\n山\t名詞\nEOS\n");
        assert_eq!(line, 2);
    }

    #[test]
    fn refuses_an_unknown_arc_type() {
        let (line, message) = format_error("* 0 -1X \n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
        assert!(message.contains("unknown arc type 'X'"), "{message}");
    }

    #[test]
    fn refuses_a_malformed_bunsetsu_line() {
        for text in [
            "* 0\n山\t名詞\nEOS\n",
            "* 0 -1D 0/0 0.000000 extra\n山\t名詞\nEOS\n",
            "* 0  -1D\n山\t名詞\nEOS\n",
            "* x -1D \n山\t名詞\nEOS\n",
            "* 0 xD \n山\t名詞\nEOS\n",
            "* 0 -1D 0/0 abc\n山\t名詞\nEOS\n",
            "* 0 -1D 0/0 inf\n山\t名詞\nEOS\n",
        ] {
            let (line, _) = format_error(text);
            assert_eq!(line, 1, "{text:?}");
        }
    }

    #[test]
    fn refuses_a_bunsetsu_line_after_loose_morphemes() {
        let (line, _) = format_error("山\t名詞\n* 0 -1D \n川\t名詞\nEOS\n");
        assert_eq!(line, 2);
    }

    #[test]
    fn refuses_a_malformed_morpheme_line() {
        for text in ["\t名詞\nEOS\n", "山\t\nEOS\n", "山\t名詞\tO\textra\nEOS\n"] {
            let (line, _) = format_error(text);
            assert_eq!(line, 1, "{text:?}");
        }
    }

    #[test]
    fn refuses_a_line_that_is_nothing() {
        let (line, _) = format_error("山 名詞\nEOS\n");
        assert_eq!(line, 1);
        let (line, _) = format_error("*0 -1D\n山\t名詞\nEOS\n");
        assert_eq!(line, 1);
    }

    #[test]
    fn the_reader_ends_after_an_error() {
        let mut reader = Reader::new("山\t名詞\nEOS\n\n川\t名詞\nEOS\n".as_bytes());
        assert!(matches!(reader.next(), Some(Ok(_))));
        assert!(matches!(
            reader.next(),
            Some(Err(Error::Format { line: 3, .. }))
        ));
        assert!(reader.next().is_none());
    }

    #[test]
    fn write_refuses_a_sentence_that_does_not_fit() {
        let mut sentence = parse("* 0 -1D \n山\t名詞\nEOS\n").unwrap().remove(0);
        sentence.bunsetsu[0].head = Some(3);
        let mut output = Vec::new();
        assert!(matches!(
            write(&mut output, &sentence),
            Err(Error::Structure(_))
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn write_refuses_a_morpheme_that_cannot_stand_on_a_line() {
        for morpheme in [
            LatticeMorpheme::new("", "名詞"),
            LatticeMorpheme::new("山", ""),
            LatticeMorpheme::new("山\t川", "名詞"),
            LatticeMorpheme::new("山", "名詞\n一般"),
            LatticeMorpheme::new("山", "名詞").with_named_entity(Some("B\tI".to_string())),
        ] {
            let sentence = Sentence::new(vec![morpheme], Vec::new()).unwrap();
            let mut output = Vec::new();
            assert!(matches!(
                write(&mut output, &sentence),
                Err(Error::Structure(_))
            ));
            assert!(output.is_empty());
        }
    }
}

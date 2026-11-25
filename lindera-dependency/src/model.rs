//! The model file: what a dictionary needs to be analyzed, in one file that
//! is read in place.
//!
//! A model file says what dictionary it is for, by a name and by the
//! [`Schema`] of the dictionary, and holds parts: the chunker and the
//! parser. A part can be absent.
//!
//! [`Model`] reads a file from bytes it borrows or owns, without copying its
//! tables: a file embedded with `include_bytes!`, read into a `Vec<u8>` or
//! mapped into memory by the caller is used as it is. No number is read
//! through a pointer, so the bytes can sit at any address. Bytes that are
//! not a model file are refused when the model is made; nothing fails
//! afterwards.
//!
//! [`ModelBuilder`] writes a file.
//!
//! # The format
//!
//! Version 1, little endian. A string is its length in bytes as a `u32`,
//! then its UTF-8.
//!
//! ```text
//! the magic number         8 bytes, "LDMODEL" and a zero
//! the version              u32
//! the name of the dictionary  string
//! the schema
//! the number of parts      u32
//! for each part:           its kind u32 (1 the chunker, 2 the parser),
//!                          its offset from the first byte of the file u64,
//!                          its length u64
//! the parts, one after the other, in this order, up to the end of the file
//! ```
//!
//! The chunker part: the number of columns as a `u32` and one byte for each,
//! the template as a string, the nine weights of the transitions as `f64`,
//! and a table of the feature strings, sorted, each with the weight of `B`
//! and the weight of `I` as `f64`.
//!
//! The parser part: the feature set as a string, one byte for the
//! quantization of the weights and one for how the weights of the pairs are
//! kept, the bias and the scale of the quantization as `f64`, a table of the
//! feature strings with their weights, and the weights of the pairs: a
//! table of `2^bits` weights the pairs are hashed into, or the pairs of
//! each feature with a weight of their own. The documentation of the
//! project has every field.
//!
//! # Examples
//!
//! ```
//! use std::collections::HashMap;
//!
//! use lindera_dependency::chunker::{ChunkerModel, Column, Template};
//! use lindera_dependency::lattice::LatticeMorpheme;
//! use lindera_dependency::model::{Model, ModelBuilder};
//! use lindera_dependency::Schema;
//!
//! // A chunker made by hand: a morpheme after a particle starts a bunsetsu,
//! // and anything else continues one.
//! let mut features = HashMap::new();
//! features.insert("U11:助詞-係助詞".to_string(), [1.0, 0.0]);
//! let transitions = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.5], [0.0, 0.0, 0.5]];
//! let chunker = ChunkerModel::new(
//!     Schema::ipadic(),
//!     vec![Column::Surface, Column::PartOfSpeech],
//!     Template::cabocha(),
//!     transitions,
//!     features,
//! )?;
//!
//! // The file of the dictionary, with this one part.
//! let bytes = ModelBuilder::new("ipadic", Schema::ipadic())?
//!     .chunker(&chunker)?
//!     .to_bytes();
//!
//! // Reading borrows the bytes.
//! let model = Model::new(bytes.as_slice())?;
//! assert_eq!(model.dictionary(), "ipadic");
//! model.check_dictionary("ipadic")?;
//! assert!(model.check_dictionary("unidic").is_err());
//!
//! let morphemes = [
//!     LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
//!     LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
//!     LatticeMorpheme::new("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
//! ];
//! let chunker = model.chunker().expect("the file has a chunker");
//! assert_eq!(chunker.chunk(&morphemes)?, [0..2, 2..3]);
//! # Ok::<(), lindera_dependency::Error>(())
//! ```

pub(crate) mod table;

use std::fmt;
use std::ops::Range;

use crate::bytes::{Reader, push_string, push_u32};
use crate::chunker::model::WEIGHTS_WIDTH;
use crate::chunker::{Chunker, ChunkerData, ChunkerModel, Column, Template};
use crate::error::{Error, Result};
use crate::morpheme::Morpheme;
use crate::parser::model::ParserLayout;
use crate::parser::{Parser, ParserModel, ParserPart, Quantization};
use crate::schema::Schema;
use crate::sentence::Bunsetsu;
use table::{Layout, Table};

/// The first bytes of a model file.
const MAGIC: &[u8; 8] = b"LDMODEL\0";

/// The version of the format.
pub(crate) const VERSION: u32 = 1;

/// The kind of the chunker part.
const KIND_CHUNKER: u32 = 1;

/// The kind of the parser part.
const KIND_PARSER: u32 = 2;

/// What the bytes are, in the messages of the errors.
const WHAT: &str = "model";

/// A model file, read in place.
///
/// `B` is what holds the bytes: a `&[u8]`, a `Vec<u8>`, or the memory map of
/// the caller. A model is made once and used from any number of threads.
///
/// See the [module](self) for the format and an example.
#[derive(Clone)]
pub struct Model<B> {
    /// The bytes of the file.
    bytes: B,
    /// The name of the dictionary the model is for.
    dictionary: String,
    /// The schema of the dictionary.
    schema: Schema,
    /// The chunker part, when the file has one.
    chunker: Option<ChunkerLayout>,
    /// The parser part, when the file has one.
    parser: Option<ParserLayout>,
    /// Where each part is in the bytes, with its kind.
    parts: Vec<(u32, Range<usize>)>,
}

/// The chunker part of a model: what is small is read, and the table of the
/// weights is where it is in the bytes.
#[derive(Debug, Clone, PartialEq)]
struct ChunkerLayout {
    /// The columns the template reads.
    columns: Vec<Column>,
    /// The template.
    template: Template,
    /// The weights of the transitions, as `[from][to]`.
    transitions: [[f64; 3]; 3],
    /// The table of the feature strings.
    features: Layout,
}

impl<B: AsRef<[u8]>> Model<B> {
    /// Reads a model file.
    ///
    /// The whole file is checked here, once: afterwards nothing of it is
    /// parsed again and nothing can fail.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes of the file, as [`ModelBuilder::to_bytes`]
    ///   wrote them.
    ///
    /// # Returns
    ///
    /// The model, or [`Error::Model`] for bytes that are not a model file
    /// of this version: another magic number or version, an unknown kind of
    /// part or the same kind twice, parts that are not one after the other
    /// up to the end of the bytes, bytes that end early, or a part whose
    /// content is not what the format says.
    pub fn new(bytes: B) -> Result<Self> {
        let all = bytes.as_ref();
        let mut reader = Reader::new(all, WHAT, Error::Model);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(Error::Model(
                "not a model file: the magic number is missing".to_string(),
            ));
        }
        let version = reader.u32()?;
        if version != VERSION {
            return Err(Error::Model(format!(
                "the model has the version {version}, and this crate reads {VERSION}"
            )));
        }
        let dictionary = reader.string()?;
        if dictionary.is_empty() {
            return Err(reader.error("has no name of a dictionary"));
        }
        let schema = Schema::read(&mut reader)?;

        let count = reader.u32()?;
        let mut parts: Vec<(u32, Range<usize>)> = Vec::new();
        for _ in 0..count {
            let kind = reader.u32()?;
            let offset = reader.u64()?;
            let length = reader.u64()?;
            let range = usize::try_from(offset)
                .ok()
                .zip(usize::try_from(length).ok())
                .and_then(|(offset, length)| Some(offset..offset.checked_add(length)?))
                .ok_or_else(|| reader.error("has a part past its end"))?;
            if parts.iter().any(|(other, _)| *other == kind) {
                return Err(reader.error(format_args!("has the part of kind {kind} twice")));
            }
            parts.push((kind, range));
        }
        // The parts follow the list, one after the other, and end the file:
        // a file that was cut, or that has something else in it, is not
        // read.
        let mut position = reader.position();
        for (_, range) in &parts {
            if range.start != position {
                return Err(reader.error("has parts that are not one after the other"));
            }
            position = range.end;
        }
        if position > all.len() {
            return Err(reader.error(format_args!(
                "ends early: {position} bytes expected, {} given",
                all.len()
            )));
        }
        if position < all.len() {
            return Err(reader.error(format_args!(
                "has {} bytes after its end",
                all.len() - position
            )));
        }

        let mut chunker = None;
        let mut parser = None;
        for (kind, range) in &parts {
            match *kind {
                KIND_CHUNKER => {
                    chunker = Some(ChunkerLayout::read(&all[range.clone()], range.start)?)
                }
                KIND_PARSER => parser = Some(ParserLayout::read(&all[range.clone()], range.start)?),
                _ => {
                    return Err(Error::Model(format!(
                        "the model has a part of the unknown kind {kind}"
                    )));
                }
            }
        }

        Ok(Self {
            bytes,
            dictionary,
            schema,
            chunker,
            parser,
            parts,
        })
    }

    /// The name of the dictionary the model is for.
    ///
    /// # Returns
    ///
    /// The name, as it was given to [`ModelBuilder::new`]: `ipadic`.
    pub fn dictionary(&self) -> &str {
        &self.dictionary
    }

    /// Checks that the model is for a dictionary.
    ///
    /// A model depends on the feature columns and on the segmentation of
    /// its dictionary: on the morphemes of another one it gives wrong
    /// answers without an error. What knows the dictionary of its
    /// morphemes calls this before it analyzes them.
    ///
    /// Morphemes with fewer feature columns than the schema of the model
    /// reads are refused when they are analyzed
    /// ([`Schema::check`]); morphemes of another dictionary that are wide
    /// enough are not, which is what this check is for.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the dictionary of the morphemes.
    ///
    /// # Returns
    ///
    /// `Ok(())` when it is the name of the model, or
    /// [`Error::Dictionary`].
    pub fn check_dictionary(&self, name: &str) -> Result<()> {
        if name == self.dictionary {
            Ok(())
        } else {
            Err(Error::Dictionary(format!(
                "the model is for the dictionary {:?}, and the morphemes are of {name:?}",
                self.dictionary
            )))
        }
    }

    /// The schema of the dictionary the model is for.
    ///
    /// # Returns
    ///
    /// The schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// The chunker of the model.
    ///
    /// # Returns
    ///
    /// A chunker that reads its weights on the bytes of the model, or
    /// `None` when the file has no chunker part.
    pub fn chunker(&self) -> Option<Chunker<ChunkerPart<'_>>> {
        let layout = self.chunker.as_ref()?;
        Some(Chunker::new(ChunkerPart {
            schema: &self.schema,
            layout,
            features: layout.features.table(self.bytes.as_ref()),
        }))
    }

    /// The parser of the model.
    ///
    /// # Returns
    ///
    /// A parser whose classifier reads its weights on the bytes of the
    /// model, or `None` when the file has no parser part.
    pub fn parser(&self) -> Option<Parser<ParserPart<'_>>> {
        let layout = self.parser.as_ref()?;
        Some(Parser::new(
            self.schema.clone(),
            layout.feature_set().clone(),
            layout.part(self.bytes.as_ref()),
        ))
    }

    /// Analyzes a sentence: segments its morphemes into bunsetsu with the
    /// chunker of the model, and finds the head of each with its parser.
    ///
    /// To analyze many sentences, take [`Model::chunker`] and
    /// [`Model::parser`] once and use them for each: this makes both for
    /// every call.
    ///
    /// # Arguments
    ///
    /// * `morphemes` - The morphemes of the sentence.
    ///
    /// # Returns
    ///
    /// The bunsetsu with their heads, as [`Parser::parse`] gives them; none
    /// for an empty sentence. [`Error::Model`] when the file has no chunker
    /// or no parser, and [`Error::Dictionary`] for a morpheme with fewer
    /// feature columns than the schema reads.
    pub fn analyze<M: Morpheme>(&self, morphemes: &[M]) -> Result<Vec<Bunsetsu>> {
        let chunker = self
            .chunker()
            .ok_or_else(|| Error::Model("the model has no chunker".to_string()))?;
        let parser = self
            .parser()
            .ok_or_else(|| Error::Model("the model has no parser".to_string()))?;
        parser.parse(morphemes, &chunker.chunk(morphemes)?)
    }

    /// The bytes of the file.
    ///
    /// # Returns
    ///
    /// The bytes the model was made from.
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl<B: AsRef<[u8]>> fmt::Debug for Model<B> {
    /// Writes what the model is for and what it has, not its bytes.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("dictionary", &self.dictionary)
            .field("bytes", &self.bytes.as_ref().len())
            .field("chunker", &self.chunker.is_some())
            .field("parser", &self.parser.is_some())
            .finish()
    }
}

impl ChunkerLayout {
    /// Reads and checks a chunker part.
    ///
    /// # Arguments
    ///
    /// * `part` - The bytes of the part.
    /// * `base` - The position of the first byte of the part in the bytes
    ///   of the model.
    ///
    /// # Returns
    ///
    /// The layout, or [`Error::Model`] for bytes that are not a chunker
    /// part: an unknown column, a template that cannot be read or that
    /// reads more columns than are given, a weight that is not finite, a
    /// table that is not one, or bytes after the table.
    fn read(part: &[u8], base: usize) -> Result<Self> {
        let mut reader = Reader::new(part, WHAT, Error::Model);
        let count = reader.u32()? as usize;
        let mut columns = Vec::new();
        for &code in reader.take(count)? {
            columns.push(
                Column::from_code(code)
                    .ok_or_else(|| reader.error(format_args!("has the unknown column {code}")))?,
            );
        }

        let template = Template::parse(&reader.string()?).map_err(|error| {
            reader.error(format_args!("has a template that is not one: {error}"))
        })?;
        if template.columns() > columns.len() {
            return Err(reader.error(format_args!(
                "has a template that reads {} columns, and {} are given",
                template.columns(),
                columns.len()
            )));
        }

        let mut transitions = [[0.0; 3]; 3];
        for weight in transitions.iter_mut().flatten() {
            *weight = reader.f64()?;
        }

        let features = Layout::read(&mut reader, WEIGHTS_WIDTH)?;
        if reader.remaining() != 0 {
            return Err(reader.error(format_args!(
                "has {} bytes after its chunker",
                reader.remaining()
            )));
        }

        let table = features.table(part);
        let finite = transitions
            .iter()
            .flatten()
            .all(|weight| weight.is_finite())
            && (0..table.len()).all(|index| {
                weights(table.value(index))
                    .is_some_and(|weights| weights.iter().all(|weight| weight.is_finite()))
            });
        if !finite {
            return Err(reader.error("has a weight that is not finite"));
        }
        Ok(Self {
            columns,
            template,
            transitions,
            features: features.shifted(base),
        })
    }
}

/// The two weights of a feature string, from the bytes of its value.
///
/// # Arguments
///
/// * `value` - The value of the string in the table.
///
/// # Returns
///
/// The weight of `B` and the weight of `I`, or `None` for a value that has
/// not the width of two weights.
fn weights(value: &[u8]) -> Option<[f64; 2]> {
    let begin = value.get(..8)?.try_into().ok()?;
    let inside = value.get(8..16)?.try_into().ok()?;
    Some([f64::from_le_bytes(begin), f64::from_le_bytes(inside)])
}

/// The chunker part of a model file, read on the bytes of the file.
///
/// It is what [`Model::chunker`] gives a [`Chunker`] to read.
#[derive(Debug, Clone, Copy)]
pub struct ChunkerPart<'a> {
    /// The schema of the model.
    schema: &'a Schema,
    /// What is small of the part.
    layout: &'a ChunkerLayout,
    /// The table of the feature strings.
    features: Table<'a>,
}

impl ChunkerPart<'_> {
    /// The number of feature strings of the part.
    ///
    /// # Returns
    ///
    /// The number of strings.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// The number of weights of the part that are not 0: those of the
    /// feature strings and those of the transitions.
    ///
    /// # Returns
    ///
    /// The number of weights.
    pub fn weight_count(&self) -> usize {
        let of_features: usize = (0..self.features.len())
            .filter_map(|index| weights(self.features.value(index)))
            .map(|weights| weights.iter().filter(|weight| **weight != 0.0).count())
            .sum();
        let of_transitions = self
            .layout
            .transitions
            .iter()
            .flatten()
            .filter(|weight| **weight != 0.0)
            .count();
        of_features + of_transitions
    }
}

impl ChunkerData for ChunkerPart<'_> {
    fn schema(&self) -> &Schema {
        self.schema
    }

    fn columns(&self) -> &[Column] {
        &self.layout.columns
    }

    fn template(&self) -> &Template {
        &self.layout.template
    }

    fn transition(&self, from: usize, to: usize) -> f64 {
        self.layout
            .transitions
            .get(from)
            .and_then(|row| row.get(to))
            .copied()
            .unwrap_or(0.0)
    }

    fn feature(&self, feature: &str) -> Option<[f64; 2]> {
        weights(self.features.value(self.features.find(feature)?))
    }
}

/// Writes a model file.
///
/// See the [module](self) for an example.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelBuilder {
    /// The name of the dictionary.
    dictionary: String,
    /// The schema of the dictionary.
    schema: Schema,
    /// The bytes of the chunker part, when one was given.
    chunker: Option<Vec<u8>>,
    /// The bytes of the parser part, when one was given.
    parser: Option<Vec<u8>>,
}

impl ModelBuilder {
    /// Starts a model file.
    ///
    /// # Arguments
    ///
    /// * `dictionary` - The name of the dictionary the model is for, as
    ///   what uses the model names the dictionary of its morphemes:
    ///   `ipadic`.
    /// * `schema` - The schema of the dictionary.
    ///
    /// # Returns
    ///
    /// The builder, or [`Error::Model`] for an empty name.
    pub fn new(dictionary: &str, schema: Schema) -> Result<Self> {
        if dictionary.is_empty() {
            return Err(Error::Model(
                "a model needs the name of its dictionary".to_string(),
            ));
        }
        Ok(Self {
            dictionary: dictionary.to_string(),
            schema,
            chunker: None,
            parser: None,
        })
    }

    /// Starts a model file with the dictionary, the schema and the parts
    /// of another one, to add a part to it or to replace one.
    ///
    /// # Arguments
    ///
    /// * `model` - The model file to start from.
    ///
    /// # Returns
    ///
    /// The builder, with the bytes of the parts of the model as they are.
    pub fn from_model<B: AsRef<[u8]>>(model: &Model<B>) -> Self {
        let part = |kind: u32| {
            model
                .parts
                .iter()
                .find(|(other, _)| *other == kind)
                .and_then(|(_, range)| model.bytes.as_ref().get(range.clone()))
                .map(<[u8]>::to_vec)
        };
        Self {
            dictionary: model.dictionary.clone(),
            schema: model.schema.clone(),
            chunker: part(KIND_CHUNKER),
            parser: part(KIND_PARSER),
        }
    }

    /// Gives the file its chunker part.
    ///
    /// # Arguments
    ///
    /// * `model` - The model of the chunker.
    ///
    /// # Returns
    ///
    /// The builder, or [`Error::Model`] when the chunker was not trained
    /// with the schema of the file.
    pub fn chunker(mut self, model: &ChunkerModel) -> Result<Self> {
        if model.schema() != &self.schema {
            return Err(Error::Model(
                "the chunker has another schema than the model".to_string(),
            ));
        }
        let mut part = Vec::new();
        model.write_part(&mut part);
        self.chunker = Some(part);
        Ok(self)
    }

    /// Gives the file its parser part.
    ///
    /// # Arguments
    ///
    /// * `model` - The model of the parser.
    /// * `quantization` - How its weights are written.
    ///
    /// # Returns
    ///
    /// The builder, or [`Error::Model`] when the parser was not trained
    /// with the schema of the file.
    pub fn parser(mut self, model: &ParserModel, quantization: Quantization) -> Result<Self> {
        if model.schema() != &self.schema {
            return Err(Error::Model(
                "the parser has another schema than the model".to_string(),
            ));
        }
        let mut part = Vec::new();
        model.write_part(&mut part, quantization);
        self.parser = Some(part);
        Ok(self)
    }

    /// Writes the file.
    ///
    /// The same parts give the same bytes.
    ///
    /// # Returns
    ///
    /// The bytes, which [`Model::new`] reads.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        push_u32(&mut bytes, VERSION);
        push_string(&mut bytes, &self.dictionary);
        self.schema.write(&mut bytes);

        let parts: Vec<(u32, &[u8])> = [(KIND_CHUNKER, &self.chunker), (KIND_PARSER, &self.parser)]
            .into_iter()
            .filter_map(|(kind, part)| Some((kind, part.as_deref()?)))
            .collect();
        push_u32(&mut bytes, parts.len() as u32);
        // Each entry of the list is a kind and two numbers of 8 bytes.
        let mut offset = bytes.len() + parts.len() * 20;
        for (kind, part) in &parts {
            push_u32(&mut bytes, *kind);
            bytes.extend_from_slice(&(offset as u64).to_le_bytes());
            bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
            offset += part.len();
        }
        for (_, part) in &parts {
            bytes.extend_from_slice(part);
        }
        bytes
    }
}

#[cfg(test)]
// A sentence that is one bunsetsu is compared with `[0..n]`: an array of one
// range, which is what is meant.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;
    use crate::chunker::model::tests::model as chunker_model;
    use crate::lattice::LatticeMorpheme;
    use crate::schema::Pattern;

    /// A chunker for which a particle continues a bunsetsu and anything
    /// else starts one, with weights that are told apart.
    fn chunker() -> ChunkerModel {
        chunker_model(
            &[
                ("U12:助詞-係助詞", [0.0, 1.0]),
                ("U12:助詞-格助詞-一般", [0.0, 1.5]),
                ("U02:は", [-0.25, 0.125]),
            ],
            [[0.0, 0.25, 0.0], [0.0, -0.5, 0.0625], [0.0, 0.125, 0.0]],
        )
    }

    /// The bytes of a model file of IPADIC with the chunker.
    fn bytes() -> Vec<u8> {
        ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .to_bytes()
    }

    /// The message of the error of bytes that are not a model file.
    fn error(bytes: &[u8]) -> String {
        let error = Model::new(bytes).unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        error.to_string()
    }

    /// The position of the number of parts in [`bytes`]: after the magic
    /// number, the version, the name and the schema.
    fn list() -> usize {
        let mut schema = Vec::new();
        Schema::ipadic().write(&mut schema);
        8 + 4 + 4 + "ipadic".len() + schema.len()
    }

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

    #[test]
    fn a_file_gives_back_what_was_written() {
        let schema = Schema {
            part_of_speech: 1..3,
            conjugation_type: None,
            conjugation_form: Some(7),
            base_form: None,
            // The patterns and the symbols go to the bytes too.
            function_words: vec![Pattern::new(&["助詞"])],
            not_heads: vec![Pattern::new(&["名詞", "非自立"]).after(Pattern::default())],
            particles: vec![Pattern::new(&["助詞", "格助詞"])],
            lexicalized: vec![Pattern::new(&["副詞"]), Pattern::new(&["連体詞"])],
            symbols: Schema::ipadic().symbols,
        };
        let transitions = [
            [0.0, 0.25, -3.0],
            [1e-300, -0.5, 1.0],
            [0.75, f64::MAX, -0.0],
        ];
        let features = [
            ("U00:彼", [-1.5, 2.0]),
            ("U01:_B-1/連用形", [0.1, -0.2]),
            ("U00:", [0.0, 3.0]),
        ];
        let original = ChunkerModel::new(
            schema.clone(),
            vec![
                Column::Surface,
                Column::PartOfSpeech,
                Column::ConjugationForm,
            ],
            Template::parse("U00:%x[0,0]\nU01:%x[-1,1]/%x[0,2]\nB\n").unwrap(),
            transitions,
            features
                .iter()
                .map(|(feature, weights)| (feature.to_string(), *weights))
                .collect(),
        )
        .unwrap();
        let builder = ModelBuilder::new("a dictionary", schema.clone())
            .unwrap()
            .chunker(&original)
            .unwrap();
        let bytes = builder.to_bytes();
        assert_eq!(&bytes[..8], b"LDMODEL\0");
        assert_eq!(bytes[8..12], [1, 0, 0, 0]);
        // The same parts give the same bytes, whatever the order of the
        // table of the chunker in memory.
        assert_eq!(builder.to_bytes(), bytes);
        assert_eq!(
            ModelBuilder::new("a dictionary", schema.clone())
                .unwrap()
                .chunker(&original.clone())
                .unwrap()
                .to_bytes(),
            bytes
        );

        let model = Model::new(bytes.as_slice()).unwrap();
        assert_eq!(model.dictionary(), "a dictionary");
        assert_eq!(model.schema(), &schema);
        assert_eq!(model.as_bytes(), bytes);
        let chunker = model.chunker().unwrap();
        let part = chunker.model();
        assert_eq!(part.schema(), &schema);
        assert_eq!(part.columns(), original.columns());
        assert_eq!(part.template(), original.template());
        for from in 0..4 {
            for to in 0..4 {
                let (read, written) = (part.transition(from, to), original.transition(from, to));
                assert_eq!(read.to_bits(), written.to_bits(), "{from} {to}");
            }
        }
        for (feature, weights) in features {
            assert_eq!(part.feature(feature), Some(weights), "{feature}");
        }
        for missing in ["U00", "U00:彼は", "U02:", ""] {
            assert_eq!(part.feature(missing), None, "{missing:?}");
        }
        assert_eq!(part.feature_count(), 3);
        assert_eq!(part.feature_count(), original.feature_count());
        assert_eq!(part.weight_count(), original.weight_count());
    }

    #[test]
    fn a_file_without_a_part_is_read() {
        let bytes = ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .to_bytes();
        assert_eq!(bytes.len(), list() + 4);
        let model = Model::new(bytes).unwrap();
        assert_eq!(model.dictionary(), "ipadic");
        assert_eq!(model.schema(), &Schema::ipadic());
        assert!(model.chunker().is_none());
    }

    #[test]
    fn the_chunker_of_a_file_chunks_as_the_model_in_memory() {
        let in_memory = Chunker::new(chunker());
        let model = Model::new(bytes()).unwrap();
        let on_bytes = model.chunker().unwrap();
        let morphemes = morphemes();
        assert_eq!(on_bytes.chunk(&morphemes).unwrap(), [0..2, 2..4, 4..5]);
        for range in [0..5, 0..1, 1..4, 1..2, 3..5, 0..0] {
            let part = &morphemes[range.clone()];
            assert_eq!(
                on_bytes.labels(&on_bytes.features(part)),
                in_memory.labels(&in_memory.features(part)),
                "{range:?}"
            );
            assert_eq!(
                on_bytes.chunk(part).unwrap(),
                in_memory.chunk(part).unwrap()
            );
        }
    }

    #[test]
    fn the_bytes_are_read_at_any_address() {
        let bytes = bytes();
        let morphemes = morphemes();
        // The same file at eight addresses in a row: whatever the alignment
        // of the first one, the others have every other one.
        for shift in 0..8 {
            let mut buffer = vec![0xa5; shift];
            buffer.extend_from_slice(&bytes);
            let model = Model::new(&buffer[shift..]).unwrap();
            let chunker = model.chunker().unwrap();
            assert_eq!(chunker.chunk(&morphemes).unwrap(), [0..2, 2..4, 4..5]);
            assert_eq!(chunker.model().feature("U02:は"), Some([-0.25, 0.125]));
            assert_eq!(chunker.model().transition(1, 2), 0.0625);
        }
    }

    #[test]
    fn a_model_holds_its_bytes_or_borrows_them() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Model<Vec<u8>>>();
        assert_send_sync::<Model<&'static [u8]>>();

        // Owned: the model can be returned from where the bytes were read.
        fn owned() -> Model<Vec<u8>> {
            Model::new(bytes()).unwrap()
        }
        let model = owned();
        let morphemes = morphemes();
        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    let chunker = model.chunker().unwrap();
                    assert_eq!(chunker.chunk(&morphemes).unwrap(), [0..2, 2..4, 4..5]);
                });
            }
        });

        // Its description names what it is for, and not its bytes.
        assert_eq!(
            format!("{model:?}"),
            format!(
                "Model {{ dictionary: \"ipadic\", bytes: {}, chunker: true, parser: false }}",
                model.as_bytes().len()
            )
        );
    }

    #[test]
    fn the_name_of_the_dictionary_is_checked() {
        let model = Model::new(bytes()).unwrap();
        assert!(model.check_dictionary("ipadic").is_ok());
        for other in ["unidic", "IPADIC", "ipadic-neologd", ""] {
            let error = model.check_dictionary(other).unwrap_err();
            assert!(matches!(error, Error::Dictionary(_)), "{error:?}");
            assert_eq!(
                error.to_string(),
                format!(
                    "the model is for the dictionary \"ipadic\", and the morphemes are of {other:?}"
                )
            );
        }
    }

    #[test]
    fn the_builder_refuses_no_name_and_another_schema() {
        assert_eq!(
            ModelBuilder::new("", Schema::ipadic())
                .unwrap_err()
                .to_string(),
            "a model needs the name of its dictionary"
        );
        let mut other = Schema::ipadic();
        other.base_form = None;
        let error = ModelBuilder::new("other", other)
            .unwrap()
            .chunker(&chunker())
            .unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "the chunker has another schema than the model"
        );
    }

    #[test]
    fn refuses_bytes_that_are_not_a_model_file() {
        let bytes = bytes();

        let mut other = bytes.clone();
        other[0] = b'X';
        assert_eq!(
            error(&other),
            "not a model file: the magic number is missing"
        );
        // A chunker model of before the model file.
        assert_eq!(
            error(b"LDCHUNK\0\x03\0\0\0"),
            "not a model file: the magic number is missing"
        );
        for version in [0, 2, 3] {
            let mut other = bytes.clone();
            other[8] = version;
            assert_eq!(
                error(&other),
                format!("the model has the version {version}, and this crate reads 1")
            );
        }

        // Cut at any length, the file is refused.
        for length in 0..bytes.len() {
            let message = error(&bytes[..length]);
            assert!(
                message.starts_with("the model ends early"),
                "{length}: {message}"
            );
        }
        let mut other = bytes.clone();
        other.push(0);
        assert_eq!(error(&other), "the model has 1 bytes after its end");

        // The name has its length after the version.
        let mut other = bytes.clone();
        other[12] = 0;
        other.drain(16..22);
        assert_eq!(error(&other), "the model has no name of a dictionary");
    }

    #[test]
    fn refuses_a_list_of_parts_that_is_not_one() {
        let bytes = bytes();
        let list = list();
        // One part: the chunker, right after the list, up to the end.
        assert_eq!(bytes[list..list + 8], [1, 0, 0, 0, 1, 0, 0, 0]);
        let offset = list + 4 + 20;
        assert_eq!(bytes[list + 8..list + 16], (offset as u64).to_le_bytes());
        let length = (bytes.len() - offset) as u64;
        assert_eq!(bytes[list + 16..list + 24], length.to_le_bytes());

        let mut other = bytes.clone();
        other[list + 4] = 3;
        assert_eq!(error(&other), "the model has a part of the unknown kind 3");

        // The part starts one byte later, or one byte earlier.
        for start in [offset + 1, offset - 1] {
            let mut other = bytes.clone();
            other[list + 8..list + 16].copy_from_slice(&(start as u64).to_le_bytes());
            assert_eq!(
                error(&other),
                "the model has parts that are not one after the other"
            );
        }

        // A part whose end is past any number.
        let mut other = bytes.clone();
        other[list + 16..list + 24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(error(&other), "the model has a part past its end");

        // A part that is longer than the bytes left, and one that is
        // shorter.
        let mut other = bytes.clone();
        other[list + 16..list + 24].copy_from_slice(&(length + 1).to_le_bytes());
        assert_eq!(
            error(&other),
            format!(
                "the model ends early: {} bytes expected, {} given",
                bytes.len() + 1,
                bytes.len()
            )
        );
        let mut other = bytes.clone();
        other[list + 16..list + 24].copy_from_slice(&(length - 1).to_le_bytes());
        assert_eq!(error(&other), "the model has 1 bytes after its end");

        // The chunker twice: a second entry, of no length.
        let mut other = bytes[..list].to_vec();
        push_u32(&mut other, 2);
        for (start, length) in [(offset + 20, length), (offset + 20 + length as usize, 0)] {
            push_u32(&mut other, KIND_CHUNKER);
            other.extend_from_slice(&(start as u64).to_le_bytes());
            other.extend_from_slice(&length.to_le_bytes());
        }
        other.extend_from_slice(&bytes[offset..]);
        assert_eq!(error(&other), "the model has the part of kind 1 twice");
    }

    /// The bytes of the chunker part of [`chunker`].
    fn part() -> Vec<u8> {
        let mut part = Vec::new();
        chunker().write_part(&mut part);
        part
    }

    /// The message of the error of bytes that are not a chunker part.
    fn part_error(part: &[u8]) -> String {
        let error = ChunkerLayout::read(part, 0).unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        error.to_string()
    }

    #[test]
    fn refuses_a_chunker_part_that_is_not_one() {
        let part = part();
        assert!(ChunkerLayout::read(&part, 0).is_ok());
        // The count of the columns, then one code each.
        assert_eq!(part[..6], [2, 0, 0, 0, 0, 1]);

        let mut other = part.clone();
        other[5] = 9;
        assert_eq!(part_error(&other), "the model has the unknown column 9");

        // One column for a template that reads two.
        let mut other = part.clone();
        other[0] = 1;
        other.remove(5);
        assert_eq!(
            part_error(&other),
            "the model has a template that reads 2 columns, and 1 are given"
        );

        // The template is a string after the columns. Its first line is a
        // unigram line, and without its `U` it is no line of a template.
        assert_eq!(&part[10..14], b"U00:");
        let mut other = part.clone();
        other[10] = b'X';
        let message = part_error(&other);
        assert!(
            message.starts_with("the model has a template that is not one: "),
            "{message}"
        );

        // A weight that is not a number. The table ends with the eight
        // slots of its three strings; the 8 bytes before them are the
        // weight of `I` of the last feature string. The transitions are the
        // 72 bytes before the table.
        let mut other = part.clone();
        let last = other.len() - 8 * 4 - 8;
        assert_eq!(other[last..last + 8], 1.5f64.to_le_bytes());
        other[last..last + 8].copy_from_slice(&f64::NAN.to_le_bytes());
        assert_eq!(
            part_error(&other),
            "the model has a weight that is not finite"
        );
        let template = u32::from_le_bytes(part[6..10].try_into().unwrap()) as usize;
        let transitions = 10 + template;
        assert_eq!(
            part[transitions + 8..transitions + 16],
            0.25f64.to_le_bytes()
        );
        let mut other = part.clone();
        other[transitions + 8..transitions + 16].copy_from_slice(&f64::INFINITY.to_le_bytes());
        assert_eq!(
            part_error(&other),
            "the model has a weight that is not finite"
        );

        // The table has values of 16 bytes.
        let table = transitions + 72;
        assert_eq!(
            part[table..table + 12],
            [3, 0, 0, 0, 16, 0, 0, 0, 8, 0, 0, 0]
        );
        let mut other = part.clone();
        other[table + 4] = 8;
        assert_eq!(
            part_error(&other),
            "the model has a table with values of 8 bytes, and 16 are expected"
        );

        let mut other = part.clone();
        other.push(0);
        assert_eq!(
            part_error(&other),
            "the model has 1 bytes after its chunker"
        );
        for length in 0..part.len() {
            let message = part_error(&part[..length]);
            assert!(
                message.starts_with("the model ends early"),
                "{length}: {message}"
            );
        }
    }

    #[test]
    fn no_changed_byte_makes_reading_or_chunking_panic() {
        let bytes = bytes();
        let morphemes = morphemes();
        let mut read = 0;
        for position in 0..bytes.len() {
            for value in [0, 1, 0x7f, 0x80, 0xff, bytes[position] ^ 1] {
                if value == bytes[position] {
                    continue;
                }
                let mut other = bytes.clone();
                other[position] = value;
                // Either the file is refused, or it is a file with other
                // content, which chunks without a panic.
                if let Ok(model) = Model::new(other) {
                    read += 1;
                    if let Some(chunker) = model.chunker() {
                        let _ = chunker.chunk(&morphemes);
                        let _ = chunker.model().weight_count();
                    }
                }
            }
        }
        // A change of a weight, for one, gives another file that is read.
        assert!(read > 0);
    }
}

/// A model file with the parser part.
#[cfg(test)]
// A sentence that is one bunsetsu is given as `[0..n]`: an array of one
// range, which is what is meant.
#[allow(clippy::single_range_in_vec_init)]
mod parser_tests {
    use super::*;
    use crate::chunker::model::tests::model as chunker_model;
    use crate::lattice::LatticeMorpheme;
    use crate::parser::{Classifier, FeatureSet, PairWeights};

    /// A chunker for which a particle continues a bunsetsu and anything
    /// else starts one.
    fn chunker() -> ChunkerModel {
        chunker_model(
            &[
                ("U12:助詞-係助詞", [0.0, 1.0]),
                ("U12:助詞-格助詞-一般", [0.0, 1.0]),
            ],
            [[0.0; 3]; 3],
        )
    }

    /// A parser for which a bunsetsu that ends in を depends on the next
    /// one, and nothing else is attached before the end of the sentence.
    fn parser() -> ParserModel {
        ParserModel::new(
            Schema::ipadic(),
            FeatureSet::cabocha(),
            -1.0,
            vec![("DIST:1".to_string(), 0.0), ("SFS:を".to_string(), 0.0)],
            PairWeights::Exact(vec![((0, 1), 2.0)]),
        )
        .unwrap()
    }

    /// The bytes of a model file of IPADIC with the two parts.
    fn bytes(quantization: Quantization) -> Vec<u8> {
        ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .parser(&parser(), quantization)
            .unwrap()
            .to_bytes()
    }

    /// 彼 は / 本 を / 読む / 人.
    fn morphemes() -> Vec<LatticeMorpheme> {
        vec![
            LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
            LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
            LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
            LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
            LatticeMorpheme::new("読む", "動詞,自立,*,*,五段・マ行,基本形,読む,ヨム,ヨム"),
            LatticeMorpheme::new("人", "名詞,一般,*,*,*,*,人,ヒト,ヒト"),
        ]
    }

    #[test]
    fn a_file_with_both_parts_analyzes_a_sentence() {
        for quantization in [Quantization::F32, Quantization::I16, Quantization::I8] {
            let bytes = bytes(quantization);
            let model = Model::new(bytes.as_slice()).unwrap();
            assert_eq!(
                format!("{model:?}"),
                format!(
                    "Model {{ dictionary: \"ipadic\", bytes: {}, chunker: true, parser: true }}",
                    bytes.len()
                )
            );
            let morphemes = morphemes();
            let analyzed = model.analyze(&morphemes).unwrap();
            let ranges: Vec<_> = analyzed.iter().map(|one| one.morphemes.clone()).collect();
            assert_eq!(ranges, [0..2, 2..4, 4..5, 5..6]);
            let heads: Vec<Option<usize>> = analyzed.iter().map(|one| one.head).collect();
            // 本 を depends on 読む, the next one; the others on the last.
            assert_eq!(heads, [Some(3), Some(2), Some(3), None]);
            // The bias and the weight of the pair, which every
            // quantization writes as it is: it is the largest weight.
            assert_eq!(analyzed[1].score, Some(1.0));
            assert_eq!(analyzed[1].positions, Some((0, 1)));

            // The same as the two steps, and as the model in memory.
            let parser_of_file = model.parser().unwrap();
            assert_eq!(parser_of_file.feature_set(), &FeatureSet::cabocha());
            assert_eq!(parser_of_file.schema(), &Schema::ipadic());
            let chunks = model.chunker().unwrap().chunk(&morphemes).unwrap();
            assert_eq!(parser_of_file.parse(&morphemes, &chunks).unwrap(), analyzed);
            let in_memory = Parser::new(Schema::ipadic(), FeatureSet::cabocha(), parser());
            assert_eq!(in_memory.parse(&morphemes, &chunks).unwrap(), analyzed);
            assert_eq!(parser_of_file.classifier().id("SFS:を"), Some(1));

            let none: [LatticeMorpheme; 0] = [];
            assert!(model.analyze(&none).unwrap().is_empty());
        }
    }

    #[test]
    fn the_parts_are_listed_in_their_order() {
        let bytes = bytes(Quantization::F32);
        let mut schema = Vec::new();
        Schema::ipadic().write(&mut schema);
        let list = 8 + 4 + 4 + "ipadic".len() + schema.len();
        assert_eq!(bytes[list..list + 4], [2, 0, 0, 0]);
        // The chunker, then the parser, one after the other to the end.
        let entry = |index: usize| {
            let start = list + 4 + 20 * index;
            let number = |from: usize| {
                u64::from_le_bytes(bytes[start + from..start + from + 8].try_into().unwrap())
                    as usize
            };
            (bytes[start], number(4), number(12))
        };
        let (first, second) = (entry(0), entry(1));
        assert_eq!(first.0, 1);
        assert_eq!(second.0, 2);
        assert_eq!(first.1, list + 4 + 40);
        assert_eq!(second.1, first.1 + first.2);
        assert_eq!(second.1 + second.2, bytes.len());
        // In whatever order the parts are given.
        let other = ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .parser(&parser(), Quantization::F32)
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .to_bytes();
        assert_eq!(other, bytes);
    }

    #[test]
    fn a_file_without_a_part_does_not_analyze() {
        let only_chunker = ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .to_bytes();
        let only_parser = ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .parser(&parser(), Quantization::I8)
            .unwrap()
            .to_bytes();
        let morphemes = morphemes();
        for (bytes, message) in [
            (only_chunker, "the model has no parser"),
            (only_parser, "the model has no chunker"),
        ] {
            let model = Model::new(bytes).unwrap();
            let error = model.analyze(&morphemes).unwrap_err();
            assert!(matches!(error, Error::Model(_)), "{error:?}");
            assert_eq!(error.to_string(), message);
            assert_eq!(model.parser().is_none(), message.ends_with("parser"));
            assert_eq!(model.chunker().is_none(), message.ends_with("chunker"));
        }
        // Morphemes of another dictionary are refused by both steps.
        let model = Model::new(bytes(Quantization::I8)).unwrap();
        let narrow = [LatticeMorpheme::new("彼", "名詞,代名詞")];
        let error = model.analyze(&narrow).unwrap_err();
        assert!(matches!(error, Error::Dictionary(_)), "{error:?}");
        let error = model.parser().unwrap().parse(&narrow, &[0..1]).unwrap_err();
        assert!(matches!(error, Error::Dictionary(_)), "{error:?}");
    }

    #[test]
    fn a_file_is_started_from_another_one() {
        let only_chunker = ModelBuilder::new("ipadic", Schema::ipadic())
            .unwrap()
            .chunker(&chunker())
            .unwrap()
            .to_bytes();
        let model = Model::new(only_chunker.as_slice()).unwrap();
        // The chunker of the file, as its bytes are, and a parser.
        let both = ModelBuilder::from_model(&model)
            .parser(&parser(), Quantization::I16)
            .unwrap()
            .to_bytes();
        assert_eq!(both, bytes(Quantization::I16));
        // Nothing added: the same file.
        assert_eq!(ModelBuilder::from_model(&model).to_bytes(), only_chunker);
        let full = Model::new(both.as_slice()).unwrap();
        assert_eq!(ModelBuilder::from_model(&full).to_bytes(), both);
        // A part given again takes the place of the one of the file.
        let again = ModelBuilder::from_model(&full)
            .parser(&parser(), Quantization::I8)
            .unwrap()
            .to_bytes();
        assert_eq!(again, bytes(Quantization::I8));
        assert_ne!(again, both);

        // The name and the schema are those of the file.
        let mut schema = Schema::ipadic();
        schema.base_form = None;
        let other = ModelBuilder::new("other", schema.clone())
            .unwrap()
            .to_bytes();
        let other = Model::new(other).unwrap();
        let copy = Model::new(ModelBuilder::from_model(&other).to_bytes()).unwrap();
        assert_eq!(copy.dictionary(), "other");
        assert_eq!(copy.schema(), &schema);
    }

    #[test]
    fn the_builder_refuses_a_parser_of_another_schema() {
        let mut other = Schema::ipadic();
        other.base_form = None;
        let error = ModelBuilder::new("other", other)
            .unwrap()
            .parser(&parser(), Quantization::F32)
            .unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "the parser has another schema than the model"
        );
    }

    #[test]
    fn a_parser_part_that_is_not_one_is_refused_with_the_file() {
        let bytes = bytes(Quantization::F32);
        // The last weight of the file is the one of the pair.
        let mut other = bytes.clone();
        let last = other.len() - 4;
        assert_eq!(other[last..], 2.0f32.to_le_bytes());
        other[last..].copy_from_slice(&f32::NAN.to_le_bytes());
        let error = Model::new(other).unwrap_err();
        assert!(matches!(error, Error::Model(_)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "the model has a weight that is not finite"
        );
        for length in 0..bytes.len() {
            assert!(Model::new(&bytes[..length]).is_err(), "{length}");
        }
    }

    #[test]
    fn no_changed_byte_makes_reading_or_analyzing_panic() {
        let bytes = bytes(Quantization::I8);
        let morphemes = morphemes();
        let mut read = 0;
        for position in 0..bytes.len() {
            for value in [0, 1, 0x7f, 0x80, 0xff, bytes[position] ^ 1] {
                if value == bytes[position] {
                    continue;
                }
                let mut other = bytes.clone();
                other[position] = value;
                if let Ok(model) = Model::new(other) {
                    read += 1;
                    let _ = model.analyze(&morphemes);
                    let _ = ModelBuilder::from_model(&model).to_bytes();
                }
            }
        }
        assert!(read > 0);
    }
}

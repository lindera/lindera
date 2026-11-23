//! Recognizing symbols, punctuation and brackets.

use super::Schema;
use super::pattern::{has_part_of_speech, owned};
use crate::bytes::{Reader, push_string, push_strings, push_u32};
use crate::error::Result;
use crate::morpheme::Morpheme;

/// What recognizes one kind of mark: parts of speech, and the characters
/// its surface is made of.
///
/// A morpheme is of the kind when its part of speech starts with one of
/// the lists of levels, or when its surface is not empty and every
/// character of it is one of `characters`. The surface is needed because a
/// dictionary does not always give a mark the part of speech of a mark:
/// IPADIC has no half-width symbols, so `,` is an unknown word and comes
/// out as 名詞,サ変接続.
///
/// The characters hold the full-width and the half-width forms side by
/// side, in the place of a normalization of the surface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Marks {
    /// The parts of speech of the kind: each one the first levels, from the
    /// coarsest, as `記号`, `読点`.
    pub part_of_speech: Vec<Vec<String>>,
    /// The characters a surface of the kind is made of.
    pub characters: String,
}

impl Marks {
    /// Makes the marks of a kind.
    ///
    /// # Arguments
    ///
    /// * `part_of_speech` - The parts of speech of the kind, each one its
    ///   first levels.
    /// * `characters` - The characters a surface of the kind is made of.
    ///
    /// # Returns
    ///
    /// The marks.
    pub fn new(part_of_speech: &[&[&str]], characters: &str) -> Self {
        Self {
            part_of_speech: part_of_speech.iter().map(|levels| owned(levels)).collect(),
            characters: characters.to_string(),
        }
    }

    /// Whether a morpheme is of the kind.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morpheme.
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether its part of speech or its surface is of the kind.
    pub(super) fn matches<M: Morpheme>(&self, schema: &Schema, morpheme: &M) -> bool {
        if self
            .part_of_speech
            .iter()
            .any(|levels| has_part_of_speech(schema, morpheme, levels))
        {
            return true;
        }
        let surface = morpheme.surface();
        !surface.is_empty()
            && surface
                .chars()
                .all(|character| self.characters.contains(character))
    }

    /// Appends the marks to the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    fn write(&self, bytes: &mut Vec<u8>) {
        push_u32(bytes, self.part_of_speech.len() as u32);
        for levels in &self.part_of_speech {
            push_strings(bytes, levels);
        }
        push_string(bytes, &self.characters);
    }

    /// Reads marks from the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the marks.
    ///
    /// # Returns
    ///
    /// The marks, or the error of bytes that end early.
    fn read(reader: &mut Reader) -> Result<Self> {
        let count = reader.u32()?;
        let mut part_of_speech = Vec::new();
        for _ in 0..count {
            part_of_speech.push(reader.strings()?);
        }
        Ok(Self {
            part_of_speech,
            characters: reader.string()?,
        })
    }
}

/// How the symbols of a dictionary are recognized, by kind.
///
/// A morpheme of any kind is a symbol ([`Schema::is_symbol`]): it is
/// neither the head nor the function word of a bunsetsu. Commas, periods
/// and brackets are kinds of their own because the parser has features for
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Symbols {
    /// The symbols that are of none of the other kinds.
    pub others: Marks,
    /// The commas (読点).
    pub commas: Marks,
    /// The periods (句点).
    pub periods: Marks,
    /// The opening brackets.
    pub open_brackets: Marks,
    /// The closing brackets.
    pub close_brackets: Marks,
}

impl Symbols {
    /// The kinds, in the order of their bytes.
    fn kinds(&self) -> [&Marks; 5] {
        [
            &self.others,
            &self.commas,
            &self.periods,
            &self.open_brackets,
            &self.close_brackets,
        ]
    }

    /// Whether a morpheme is a symbol of any kind.
    ///
    /// A surface that mixes the kinds, as `,“` (which IPADIC gives as one
    /// unknown word), is a symbol too: every character of it is of some
    /// kind.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema of the dictionary of the morpheme.
    /// * `morpheme` - The morpheme.
    ///
    /// # Returns
    ///
    /// Whether the morpheme is a symbol.
    pub(super) fn matches<M: Morpheme>(&self, schema: &Schema, morpheme: &M) -> bool {
        let kinds = self.kinds();
        if kinds.iter().any(|kind| {
            kind.part_of_speech
                .iter()
                .any(|levels| has_part_of_speech(schema, morpheme, levels))
        }) {
            return true;
        }
        let surface = morpheme.surface();
        !surface.is_empty()
            && surface
                .chars()
                .all(|character| kinds.iter().any(|kind| kind.characters.contains(character)))
    }

    /// Appends the symbols to the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to append to.
    pub(super) fn write(&self, bytes: &mut Vec<u8>) {
        for kind in self.kinds() {
            kind.write(bytes);
        }
    }

    /// Reads symbols from the bytes of a schema.
    ///
    /// # Arguments
    ///
    /// * `reader` - The bytes, at the symbols.
    ///
    /// # Returns
    ///
    /// The symbols, or the error of bytes that end early.
    pub(super) fn read(reader: &mut Reader) -> Result<Self> {
        Ok(Self {
            others: Marks::read(reader)?,
            commas: Marks::read(reader)?,
            periods: Marks::read(reader)?,
            open_brackets: Marks::read(reader)?,
            close_brackets: Marks::read(reader)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::lattice::LatticeMorpheme;

    /// Symbols with one part of speech and one or two characters per kind.
    fn symbols() -> Symbols {
        Symbols {
            others: Marks::new(&[&["記号"]], "!-"),
            commas: Marks::new(&[&["記号", "読点"]], "、,"),
            periods: Marks::new(&[&["記号", "句点"]], "。."),
            open_brackets: Marks::new(&[&["記号", "括弧開"]], "「("),
            close_brackets: Marks::new(&[&["記号", "括弧閉"]], "」)"),
        }
    }

    #[test]
    fn a_kind_is_recognized_by_the_part_of_speech_or_by_the_surface() {
        let schema = Schema::ipadic();
        let commas = symbols().commas;
        // By the part of speech, whatever the surface.
        assert!(commas.matches(&schema, &LatticeMorpheme::new("x", "記号,読点,*,*")));
        // By the surface, whatever the part of speech.
        assert!(commas.matches(&schema, &LatticeMorpheme::new(",", "名詞,サ変接続,*,*")));
        assert!(commas.matches(&schema, &LatticeMorpheme::new(",、", "名詞,サ変接続,*,*")));
        // Neither.
        assert!(!commas.matches(&schema, &LatticeMorpheme::new("x", "記号,句点,*,*")));
        assert!(!commas.matches(&schema, &LatticeMorpheme::new(".", "名詞,サ変接続,*,*")));
        // Every character has to be of the kind, and there has to be one.
        assert!(!commas.matches(&schema, &LatticeMorpheme::new(",a", "名詞,一般,*,*")));
        assert!(!commas.matches(&schema, &LatticeMorpheme::new("", "名詞,一般,*,*")));
    }

    #[test]
    fn a_symbol_is_a_morpheme_of_any_kind_or_of_characters_of_several() {
        let schema = Schema::ipadic();
        let symbols = symbols();
        for (surface, features) in [
            ("★", "記号,一般,*,*"),
            ("x", "記号,括弧閉,*,*"),
            ("!", "名詞,サ変接続,*,*"),
            (")", "名詞,サ変接続,*,*"),
            // A comma and a bracket in one unknown word.
            (",「", "名詞,サ変接続,*,*"),
        ] {
            let morpheme = LatticeMorpheme::new(surface, features);
            assert!(symbols.matches(&schema, &morpheme), "{surface}");
        }
        for (surface, features) in [
            ("%", "名詞,サ変接続,*,*"),
            ("%,", "名詞,サ変接続,*,*"),
            ("本", "名詞,一般,*,*"),
            ("", "名詞,一般,*,*"),
        ] {
            let morpheme = LatticeMorpheme::new(surface, features);
            assert!(!symbols.matches(&schema, &morpheme), "{surface}");
        }
    }

    #[test]
    fn symbols_come_back_from_their_bytes() {
        let symbols = symbols();
        let mut bytes = Vec::new();
        symbols.write(&mut bytes);
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        assert_eq!(Symbols::read(&mut reader).unwrap(), symbols);
        assert_eq!(reader.remaining(), 0);
        // Each kind is where it was: none is read into the place of
        // another.
        let mut reader = Reader::new(&bytes, "test bytes", Error::Chunker);
        let read = Symbols::read(&mut reader).unwrap();
        assert_eq!(read.others.characters, "!-");
        assert_eq!(read.commas.characters, "、,");
        assert_eq!(read.periods.characters, "。.");
        assert_eq!(read.open_brackets.characters, "「(");
        assert_eq!(read.close_brackets.characters, "」)");
    }
}

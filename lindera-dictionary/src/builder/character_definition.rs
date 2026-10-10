use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use byteorder::{ByteOrder, LittleEndian};
use encoding_rs::UTF_16LE;
use log::debug;

use crate::LinderaResult;
use crate::dictionary::character_definition::{
    CategoryData, CategoryId, CharacterDefinition, LookupTable,
};
use crate::error::LinderaErrorKind;
use crate::util::{read_file_with_encoding, write_data};

const DEFAULT_CATEGORY_NAME: &str = "DEFAULT";

fn ucs2_to_unicode(ucs2_codepoint: u16) -> LinderaResult<u32> {
    let mut buf = [0u8; 2];
    LittleEndian::write_u16(&mut buf[..], ucs2_codepoint);

    let s = UTF_16LE.decode(&buf[..]).0.into_owned();
    let chrs: Vec<char> = s.chars().collect();

    match chrs.len() {
        1 => Ok(chrs[0] as u32),
        _ => Err(LinderaErrorKind::Parse
            .with_error(anyhow::anyhow!("unusual char length"))
            .add_context(format!(
                "UCS2 codepoint 0x{:04x} resulted in {} characters",
                ucs2_codepoint,
                chrs.len()
            ))),
    }
}

fn parse_hex_codepoint(s: &str) -> LinderaResult<u32> {
    let removed_0x = s.trim_start_matches("0x");
    let ucs2_codepoint = u16::from_str_radix(removed_0x, 16).map_err(|err| {
        LinderaErrorKind::Parse
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Invalid hexadecimal codepoint: '{s}'"))
    })?;

    ucs2_to_unicode(ucs2_codepoint)
}

#[derive(Debug)]
pub struct CharacterDefinitionBuilder {
    encoding: Cow<'static, str>,
    category_definition: Vec<CategoryData>,
    category_index: HashMap<String, CategoryId>,
    char_ranges: Vec<(u32, u32, Vec<CategoryId>)>,
}

/// Options for [`CharacterDefinitionBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Debug, Default)]
pub struct CharacterDefinitionBuilderOptions {
    encoding: Option<Cow<'static, str>>,
    category_definition: Option<Vec<CategoryData>>,
    category_index: Option<HashMap<String, CategoryId>>,
    char_ranges: Option<Vec<(u32, u32, Vec<CategoryId>)>>,
}

impl CharacterDefinitionBuilderOptions {
    pub fn encoding(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.encoding = Some(value.into());
        self
    }

    pub fn category_definition(&mut self, value: Vec<CategoryData>) -> &mut Self {
        self.category_definition = Some(value);
        self
    }

    pub fn category_index(&mut self, value: HashMap<String, CategoryId>) -> &mut Self {
        self.category_index = Some(value);
        self
    }

    pub fn char_ranges(&mut self, value: Vec<(u32, u32, Vec<CategoryId>)>) -> &mut Self {
        self.char_ranges = Some(value);
        self
    }

    pub fn builder(&self) -> CharacterDefinitionBuilder {
        CharacterDefinitionBuilder {
            encoding: self.encoding.clone().unwrap_or_else(|| "UTF-8".into()),
            category_definition: self.category_definition.clone().unwrap_or_default(),
            category_index: self.category_index.clone().unwrap_or_default(),
            char_ranges: self.char_ranges.clone().unwrap_or_default(),
        }
    }
}

impl CharacterDefinitionBuilder {
    pub fn category_id(&mut self, category_name: &str) -> CategoryId {
        let num_categories = self.category_index.len();
        *self
            .category_index
            .entry(category_name.to_string())
            .or_insert(CategoryId(num_categories))
    }

    fn parse_range(&mut self, line: &str) -> LinderaResult<()> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let range_bounds: Vec<&str> = fields[0].split("..").collect();
        let lower_bound: u32;
        let higher_bound: u32;
        match range_bounds.len() {
            1 => {
                lower_bound = parse_hex_codepoint(range_bounds[0])?;
                higher_bound = lower_bound;
            }
            2 => {
                lower_bound = parse_hex_codepoint(range_bounds[0])?;
                // the right bound is included in the file.
                higher_bound = parse_hex_codepoint(range_bounds[1])?;
            }
            _ => {
                return Err(LinderaErrorKind::Content
                    .with_error(anyhow::anyhow!("Invalid line: {line}"))
                    .add_context(format!(
                        "Character range should have format 'START..END' or 'SINGLE', got {} parts",
                        range_bounds.len()
                    )));
            }
        }
        // A later line replaces an earlier one (see `lookup_categories`), so a
        // line without categories would silently reset the range to DEFAULT.
        // MeCab rejects such a line as a format error.
        if fields.len() < 2 {
            return Err(LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("Invalid line: {line}"))
                .add_context("Character range requires at least one category"));
        }
        let category_ids: Vec<CategoryId> = fields[1..]
            .iter()
            .map(|category| self.category_id(category))
            .collect();

        self.char_ranges
            .push((lower_bound, higher_bound, category_ids));

        Ok(())
    }

    fn parse_category(&mut self, line: &str) -> LinderaResult<()> {
        let fields = line.split_ascii_whitespace().collect::<Vec<&str>>();
        if fields.len() != 4 {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "Expected 4 fields. Got {} in {}",
                fields.len(),
                line
            )).add_context("Character category definition requires: <category_name> <invoke> <group> <length>"));
        }
        let invoke = fields[1].parse::<u32>().map_err(|err| {
            LinderaErrorKind::Parse
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Invalid 'invoke' field value '{}' for category '{}'",
                    fields[1], fields[0]
                ))
        })? == 1;
        let group = fields[2].parse::<u32>().map_err(|err| {
            LinderaErrorKind::Parse
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Invalid 'group' field value '{}' for category '{}'",
                    fields[2], fields[0]
                ))
        })? == 1;
        let length = fields[3].parse::<u32>().map_err(|err| {
            LinderaErrorKind::Parse
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Invalid 'length' field value '{}' for category '{}'",
                    fields[3], fields[0]
                ))
        })?;
        let category_data = CategoryData {
            invoke,
            group,
            length,
        };
        // force a category_id allocation
        self.category_id(fields[0]);
        self.category_definition.push(category_data);

        Ok(())
    }

    fn parse(&mut self, content: &str) -> LinderaResult<()> {
        for line in content.lines() {
            let line_str = line
                .split('#')
                .next()
                .ok_or_else(|| {
                    LinderaErrorKind::Parse
                        .with_error(anyhow::anyhow!("failed to parse line"))
                        .add_context(format!("Malformed line in character definition: '{line}'"))
                })?
                .trim();
            if line_str.is_empty() {
                continue;
            }
            if line_str.starts_with("0x") {
                self.parse_range(line_str)?;
            } else {
                self.parse_category(line_str)?;
            }
        }
        Ok(())
    }

    /// Returns the categories of the last range line that covers a code point.
    ///
    /// MeCab fills its character table line by line, so a later line that
    /// covers a code point replaces everything an earlier line said about it
    /// instead of adding to it.
    ///
    /// # Arguments
    ///
    /// * `c` - The code point to look up.
    ///
    /// # Returns
    ///
    /// The categories of that line in the order they are written, or `None`
    /// when no line covers `c`.
    fn last_covering_categories(&self, c: u32) -> Option<&[CategoryId]> {
        self.char_ranges
            .iter()
            .rev()
            .find(|(start, stop, _)| *start <= c && c <= *stop)
            .map(|(_, _, category_ids)| category_ids.as_slice())
    }

    /// Returns the id of the `DEFAULT` category, if `char.def` uses it.
    ///
    /// # Returns
    ///
    /// The id, or `None` when no line names `DEFAULT`.
    fn default_category_id(&self) -> Option<CategoryId> {
        self.category_index.get(DEFAULT_CATEGORY_NAME).copied()
    }

    /// Writes the categories of a code point into `categories_buffer`.
    ///
    /// The set comes from the last line that covers the code point, as in
    /// MeCab. Its first entry is the default category (the first category of
    /// that line, MeCab's `default_type`), which alone creates unknown-word
    /// candidates; the other categories follow in category id order (the
    /// order in which `char.def` introduces them), each once. A code point
    /// that no line covers gets `DEFAULT`.
    ///
    /// # Arguments
    ///
    /// * `c` - The code point to look up.
    /// * `categories_buffer` - Cleared, then filled with the category ids.
    fn lookup_categories(&self, c: u32, categories_buffer: &mut Vec<CategoryId>) {
        categories_buffer.clear();
        if let Some(category_ids) = self.last_covering_categories(c) {
            categories_buffer.extend_from_slice(category_ids);
            categories_buffer.sort_unstable();
            categories_buffer.dedup();
            // Move the default category, the line's first (`parse_range`
            // rejects a line without categories), to the front; the
            // categories before it shift by one and stay in id order.
            if let Some(&default_category) = category_ids.first()
                && let Some(pos) = categories_buffer
                    .iter()
                    .position(|&id| id == default_category)
            {
                categories_buffer[..=pos].rotate_right(1);
            }
        } else if let Some(default_category) = self.default_category_id() {
            categories_buffer.push(default_category);
        }
    }

    /// Writes the default category of a code point into `categories_buffer`.
    ///
    /// The default category is the first category of the last line that
    /// covers the code point (MeCab's `default_type`), or `DEFAULT` when no
    /// line covers it.
    ///
    /// # Arguments
    ///
    /// * `c` - The code point to look up.
    /// * `categories_buffer` - Cleared, then filled with at most one category id.
    fn lookup_default_category(&self, c: u32, categories_buffer: &mut Vec<CategoryId>) {
        categories_buffer.clear();
        let default_category = match self.last_covering_categories(c) {
            Some(category_ids) => category_ids.first().copied(),
            None => self.default_category_id(),
        };
        categories_buffer.extend(default_category);
    }

    /// Returns every code point at which the result of a lookup can change.
    ///
    /// # Returns
    ///
    /// The start and the end + 1 of every range line, sorted and deduplicated.
    fn range_boundaries(&self) -> Vec<u32> {
        let boundaries_set: BTreeSet<u32> = self
            .char_ranges
            .iter()
            .flat_map(|(low, high, _)| [*low, *high + 1u32])
            .collect();
        boundaries_set.into_iter().collect()
    }

    /// Builds the table that maps every code point to its categories.
    ///
    /// # Returns
    ///
    /// The lookup table stored in `char_def.bin`.
    fn build_lookup_table(&self) -> LookupTable<CategoryId> {
        LookupTable::from_fn(self.range_boundaries(), &|c, buff| {
            self.lookup_categories(c, buff)
        })
    }

    /// Builds a table that maps every code point to its default category.
    ///
    /// The default category is the first category of the last `char.def` line
    /// that covers the code point (MeCab's `default_type`), or `DEFAULT` when
    /// no line covers it. The dictionary itself does not store it; the trainer
    /// uses it for the `%t` feature. Call it after [`Self::build_from_str`] or
    /// [`Self::build`].
    ///
    /// # Returns
    ///
    /// A table whose rows hold one category id each (none when `char.def`
    /// does not use `DEFAULT` and no line covers the code point).
    pub fn build_default_category_table(&self) -> LookupTable<CategoryId> {
        LookupTable::from_fn(self.range_boundaries(), &|c, buff| {
            self.lookup_default_category(c, buff)
        })
    }

    fn get_character_definition(&self) -> CharacterDefinition {
        let mut category_names: Vec<String> = (0..self.category_index.len())
            .map(|_| String::new())
            .collect();
        for (category_name, category_id) in &self.category_index {
            category_names[category_id.0] = category_name.clone();
        }
        let mapping = self.build_lookup_table();
        CharacterDefinition::new(self.category_definition.clone(), category_names, mapping)
    }

    /// Parses `char.def` content and returns the character definition it
    /// describes.
    ///
    /// Unlike [`Self::build`], it reads no file and writes no `char_def.bin`,
    /// so other components (such as the trainer) can read `char.def` with
    /// exactly the rules the dictionary builder uses.
    ///
    /// # Arguments
    ///
    /// * `content` - The text of a `char.def` file.
    ///
    /// # Returns
    ///
    /// The character definition, or an error if a line cannot be parsed.
    pub fn build_from_str(&mut self, content: &str) -> LinderaResult<CharacterDefinition> {
        self.parse(content)?;
        Ok(self.get_character_definition())
    }

    /// Reads `char.def` from `input_dir` and writes `char_def.bin` to
    /// `output_dir`.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - The dictionary source directory holding `char.def`.
    /// * `output_dir` - The directory to write `char_def.bin` to.
    ///
    /// # Returns
    ///
    /// The character definition that was written, or an error if `char.def`
    /// cannot be read or parsed, or the output cannot be written.
    pub fn build(
        &mut self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {char_def_path:?}");
        let char_def = read_file_with_encoding(&char_def_path, &self.encoding)?;

        let char_definitions = self.build_from_str(&char_def)?;

        let mut chardef_buffer = Vec::new();
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&char_definitions).map_err(|err| {
            LinderaErrorKind::Serialize
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to serialize character definition data")
        })?;
        chardef_buffer.write_all(&bytes).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write character definition data to buffer")
        })?;

        let wtr_chardef_path = output_dir.join(Path::new("char_def.bin"));
        let mut wtr_chardef =
            io::BufWriter::new(File::create(&wtr_chardef_path).map_err(|err| {
                LinderaErrorKind::Io
                    .with_error(anyhow::anyhow!(err))
                    .add_context(format!(
                        "Failed to create character definition output file: {wtr_chardef_path:?}"
                    ))
            })?);

        write_data(&chardef_buffer, &mut wtr_chardef)?;

        wtr_chardef.flush().map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context(format!(
                    "Failed to flush character definition output file: {wtr_chardef_path:?}"
                ))
        })?;

        Ok(char_definitions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The overlapping lines of IPADIC's `char.def` that #1095 is about,
    /// in the same order as there.
    const OVERLAPPING_CHAR_DEF: &str = "\
DEFAULT 0 1 0
SPACE 0 1 0
KANJI 0 0 2
SYMBOL 1 1 0
ALPHA 1 1 0
KANJINUMERIC 1 1 0

0x0020 SPACE
0x00D0 SPACE  # a typo for 0x000D in IPADIC
0x00C0..0x00FF ALPHA
0x3005 KANJI
0x3007 KANJI
0x4E00..0x9FA5 KANJI
0x4E00 KANJINUMERIC KANJI
0x3000..0x303F SYMBOL
0x3007 SYMBOL KANJINUMERIC
";

    fn build(content: &str) -> (CharacterDefinitionBuilder, CharacterDefinition) {
        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        let char_def = builder.build_from_str(content).unwrap();
        (builder, char_def)
    }

    fn category_names(char_def: &CharacterDefinition, c: char) -> Vec<&str> {
        char_def
            .lookup_categories(c)
            .iter()
            .map(|id| char_def.category_name(*id))
            .collect()
    }

    fn default_category_names<'a>(
        char_def: &'a CharacterDefinition,
        table: &LookupTable<CategoryId>,
        c: char,
    ) -> Vec<&'a str> {
        table
            .eval(c as u32)
            .iter()
            .map(|id| char_def.category_name(*id))
            .collect()
    }

    #[test]
    fn test_later_line_overrides_earlier_line() {
        let (_, char_def) = build(OVERLAPPING_CHAR_DEF);

        // U+00D0 is only ALPHA, as in MeCab, so it is no longer whitespace.
        assert_eq!(category_names(&char_def, 'Ð'), vec!["ALPHA"]);
        assert_eq!(category_names(&char_def, 'À'), vec!["ALPHA"]);
        assert_eq!(category_names(&char_def, ' '), vec!["SPACE"]);
        // The SYMBOL range replaces the KANJI line of U+3005.
        assert_eq!(category_names(&char_def, '々'), vec!["SYMBOL"]);
        assert_eq!(category_names(&char_def, '「'), vec!["SYMBOL"]);
    }

    #[test]
    fn test_default_category_comes_first() {
        let (_, char_def) = build(OVERLAPPING_CHAR_DEF);

        // The line says `KANJINUMERIC KANJI`: the default category comes
        // first although char.def defines KANJI before it (#1111).
        assert_eq!(
            category_names(&char_def, '一'),
            vec!["KANJINUMERIC", "KANJI"]
        );
        assert_eq!(category_names(&char_def, '丁'), vec!["KANJI"]);
        assert_eq!(
            category_names(&char_def, '〇'),
            vec!["SYMBOL", "KANJINUMERIC"]
        );
    }

    #[test]
    fn test_other_categories_follow_in_category_id_order() {
        let (_, char_def) = build(
            "DEFAULT 0 1 0\nKANJI 0 0 2\nSYMBOL 1 1 0\nKANJINUMERIC 1 1 0\n\
             0x4E00 KANJINUMERIC SYMBOL KANJI\n0x4E01 SYMBOL KANJI SYMBOL\n",
        );

        // After the default category, the others are listed in the order in
        // which char.def defines them, each once.
        assert_eq!(
            category_names(&char_def, '一'),
            vec!["KANJINUMERIC", "KANJI", "SYMBOL"]
        );
        assert_eq!(category_names(&char_def, '丁'), vec!["SYMBOL", "KANJI"]);
    }

    /// The first category of every row is the default category that
    /// `build_default_category_table` gives, for every code point.
    #[test]
    fn test_first_category_is_the_default_category() {
        for content in [
            OVERLAPPING_CHAR_DEF,
            "DEFAULT 0 1 0\nKANJI 0 0 2\nSYMBOL 1 1 0\nKANJINUMERIC 1 1 0\n\
             0x4E00..0x4E0F KANJI\n0x4E00 KANJINUMERIC SYMBOL KANJI\n",
        ] {
            let (builder, char_def) = build(content);
            let table = builder.build_default_category_table();
            for c in (0..=0x10FFFF).filter_map(char::from_u32) {
                assert_eq!(
                    char_def.lookup_categories(c).first(),
                    table.eval(c as u32).first(),
                    "U+{:04X}",
                    c as u32
                );
            }
        }
    }

    #[test]
    fn test_uncovered_code_point_is_default() {
        let (_, char_def) = build(OVERLAPPING_CHAR_DEF);

        assert_eq!(category_names(&char_def, 'a'), vec!["DEFAULT"]);
        assert_eq!(category_names(&char_def, 'あ'), vec!["DEFAULT"]);
        assert_eq!(category_names(&char_def, '😀'), vec!["DEFAULT"]);
    }

    #[test]
    fn test_default_category_is_first_category_of_last_line() {
        let (builder, char_def) = build(OVERLAPPING_CHAR_DEF);
        let table = builder.build_default_category_table();

        assert_eq!(
            default_category_names(&char_def, &table, '一'),
            vec!["KANJINUMERIC"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, '丁'),
            vec!["KANJI"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, '〇'),
            vec!["SYMBOL"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, '々'),
            vec!["SYMBOL"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, 'Ð'),
            vec!["ALPHA"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, ' '),
            vec!["SPACE"]
        );
        assert_eq!(
            default_category_names(&char_def, &table, 'a'),
            vec!["DEFAULT"]
        );
    }

    #[test]
    fn test_default_category_table_without_default_category() {
        let (builder, _) = build("ALPHA 1 1 0\n0x0041..0x005A ALPHA\n");
        let table = builder.build_default_category_table();

        assert_eq!(table.eval('A' as u32), &[CategoryId(0)]);
        assert!(table.eval('a' as u32).is_empty());
    }

    #[test]
    fn test_range_line_without_category_is_an_error() {
        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        let result = builder.build_from_str("DEFAULT 0 1 0\n0x0041..0x005A  # no category\n");

        assert!(result.is_err());
    }

    #[test]
    fn test_build_writes_the_same_definition() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::write(input_dir.path().join("char.def"), OVERLAPPING_CHAR_DEF).unwrap();

        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        builder.build(input_dir.path(), output_dir.path()).unwrap();
        let written = std::fs::read(output_dir.path().join("char_def.bin")).unwrap();
        let loaded = CharacterDefinition::load(&written).unwrap();

        let (_, expected) = build(OVERLAPPING_CHAR_DEF);
        for c in ['Ð', ' ', '々', '一', '〇', 'a'] {
            assert_eq!(category_names(&loaded, c), category_names(&expected, c));
        }
    }
}

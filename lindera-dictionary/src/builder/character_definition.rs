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
    CategoryData, CategoryId, CharDefFormat, CharacterDefinition, LookupTable, NO_OOV_BOW,
    NO_OOV_EOW,
};
use crate::error::LinderaErrorKind;
use crate::util::{read_file_with_encoding, write_data};

const DEFAULT_CATEGORY_NAME: &str = "DEFAULT";

/// Sudachi's name for every category on a range line (#1388).
const ALL_CATEGORIES_NAME: &str = "ALL";

/// Sudachi's name of the flag [`NO_OOV_BOW`] on a range line.
const NO_OOV_BOW_NAME: &str = "NOOOVBOW";

/// Sudachi's name of the flag [`NO_OOV_EOW`] on a range line.
const NO_OOV_EOW_NAME: &str = "NOOOVEOW";

/// The names that only the Sudachi format reads on a range line, and that
/// it does not let a definition line define.
const SUDACHI_RESERVED_NAMES: [&str; 3] = [ALL_CATEGORIES_NAME, NO_OOV_BOW_NAME, NO_OOV_EOW_NAME];

/// The most categories the Sudachi format takes: the lattice intersects the
/// categories of a run as the bits of a `u64` (Sudachi has 15).
const SUDACHI_MAX_CATEGORIES: usize = 64;

/// One range line of a `char.def` in the Sudachi format.
#[derive(Debug, Clone)]
struct SudachiRange {
    /// The first code point of the range.
    low: u32,
    /// The last code point of the range, included.
    high: u32,
    /// The categories the line names, in the order written.
    categories: Vec<CategoryId>,
    /// Whether the line names `ALL`.
    all: bool,
    /// The flags the line names ([`NO_OOV_BOW`], [`NO_OOV_EOW`]).
    flags: u8,
}

/// Parses a code point of a range line in the Sudachi format, which, unlike
/// MeCab's UCS-2, can be outside the BMP (Sudachi's `char.def` maps
/// `0x1F3FB..0x1F3FE`).
///
/// # Arguments
///
/// * `source` - The name of the `char.def` content, for the message.
/// * `line_number` - The 1-based number of the line.
/// * `s` - The code point, with or without `0x`.
///
/// # Returns
///
/// The code point, or an error when it is not hexadecimal or not a Unicode
/// scalar value.
fn parse_sudachi_codepoint(source: &str, line_number: usize, s: &str) -> LinderaResult<u32> {
    u32::from_str_radix(s.trim_start_matches("0x"), 16)
        .ok()
        .filter(|&cp| char::from_u32(cp).is_some())
        .ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: line {line_number}: {s} is not a code point"
            ))
        })
}

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
    /// The format to read `char.def` in (#1388).
    format: CharDefFormat,
    category_definition: Vec<CategoryData>,
    category_index: HashMap<String, CategoryId>,
    /// The range lines in the MeCab format.
    char_ranges: Vec<(u32, u32, Vec<CategoryId>)>,
    /// The range lines in the Sudachi format.
    sudachi_ranges: Vec<SudachiRange>,
    /// The 1-based line on which each category was defined, for the error
    /// on a second definition.
    category_lines: HashMap<String, usize>,
}

/// Options for [`CharacterDefinitionBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Debug, Default)]
pub struct CharacterDefinitionBuilderOptions {
    encoding: Option<Cow<'static, str>>,
    format: Option<CharDefFormat>,
    category_definition: Option<Vec<CategoryData>>,
    category_index: Option<HashMap<String, CategoryId>>,
    char_ranges: Option<Vec<(u32, u32, Vec<CategoryId>)>>,
}

impl CharacterDefinitionBuilderOptions {
    pub fn encoding(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.encoding = Some(value.into());
        self
    }

    /// Sets the format to read `char.def` in (#1388); MeCab's by default.
    ///
    /// # Arguments
    ///
    /// * `value` - The format, from `char_def_format` in `metadata.json`.
    ///
    /// # Returns
    ///
    /// The options.
    pub fn format(&mut self, value: CharDefFormat) -> &mut Self {
        self.format = Some(value);
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
            format: self.format.unwrap_or_default(),
            category_definition: self.category_definition.clone().unwrap_or_default(),
            category_index: self.category_index.clone().unwrap_or_default(),
            char_ranges: self.char_ranges.clone().unwrap_or_default(),
            sudachi_ranges: Vec::new(),
            category_lines: HashMap::new(),
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

    /// Returns the id of a category that an earlier line defines, for a range
    /// line.
    ///
    /// MeCab checks the categories of a range line against those defined so
    /// far, so a range line cannot name a category that only a later line
    /// defines: an id given out at the range line would not match the slot
    /// of the category's data. A name that has an id but no data (from
    /// [`Self::category_id`]) is not defined either.
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the `char.def` content, for the message.
    /// * `line_number` - The 1-based number of the range line.
    /// * `category` - The category name.
    ///
    /// # Returns
    ///
    /// The id, or an error naming the source, the line and the category. In
    /// the MeCab format, the error on a name that only the Sudachi format
    /// reads (`ALL`, `NOOOVBOW`, `NOOOVEOW`) says how to select that format,
    /// and that `lindera train` does not read it (#1388).
    fn defined_category_id(
        &self,
        source: &str,
        line_number: usize,
        category: &str,
    ) -> LinderaResult<CategoryId> {
        match self.category_index.get(category) {
            Some(&category_id) if category_id.0 < self.category_definition.len() => Ok(category_id),
            _ => {
                let hint = if self.format == CharDefFormat::Mecab
                    && SUDACHI_RESERVED_NAMES.contains(&category)
                {
                    format!(
                        " ({category} is the syntax of Sudachi's char.def, which a dictionary \
                         selects with \"char_def_format\": \"sudachi\" in its metadata.json; \
                         lindera dictionary train reads MeCab's format only)"
                    )
                } else {
                    String::new()
                };
                Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "{source}: line {line_number}: category {category} is not defined; a range \
                     line can only name a category defined on an earlier line{hint}"
                )))
            }
        }
    }

    /// Parses a range line (`0xLOW[..0xHIGH] CATEGORY...`).
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the `char.def` content, for the messages.
    /// * `line_number` - The 1-based number of the line.
    /// * `line` - The line without its comment.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when a bound does not parse, the line has no
    /// category, or a category is not defined on an earlier line.
    fn parse_range(&mut self, source: &str, line_number: usize, line: &str) -> LinderaResult<()> {
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
        let category_ids = fields[1..]
            .iter()
            .map(|category| self.defined_category_id(source, line_number, category))
            .collect::<LinderaResult<Vec<CategoryId>>>()?;

        self.char_ranges
            .push((lower_bound, higher_bound, category_ids));

        Ok(())
    }

    /// Parses a range line in the Sudachi format (`0xLOW[..0xHIGH] NAME...`,
    /// #1388): the code points can be outside the BMP, and besides the
    /// categories defined on earlier lines, a name can be `ALL` (every
    /// category) or the flag `NOOOVBOW` or `NOOOVEOW`.
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the `char.def` content, for the messages.
    /// * `line_number` - The 1-based number of the line.
    /// * `line` - The line without its comment.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when a bound is not a code point, the range
    /// ends before it starts, the line names nothing, or a name is neither
    /// reserved nor a category defined on an earlier line.
    fn parse_range_sudachi(
        &mut self,
        source: &str,
        line_number: usize,
        line: &str,
    ) -> LinderaResult<()> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (low, high) = match fields[0].split("..").collect::<Vec<&str>>()[..] {
            [single] => {
                let cp = parse_sudachi_codepoint(source, line_number, single)?;
                (cp, cp)
            }
            [low, high] => (
                parse_sudachi_codepoint(source, line_number, low)?,
                parse_sudachi_codepoint(source, line_number, high)?,
            ),
            _ => {
                return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "{source}: line {line_number}: a character range should be 'START..END' or \
                     'SINGLE', got {}",
                    fields[0]
                )));
            }
        };
        if low > high {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: line {line_number}: the range {} ends before it starts",
                fields[0]
            )));
        }
        if fields.len() < 2 {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: line {line_number}: a character range requires at least one category"
            )));
        }
        let mut range = SudachiRange {
            low,
            high,
            categories: Vec::new(),
            all: false,
            flags: 0,
        };
        for &name in &fields[1..] {
            match name {
                ALL_CATEGORIES_NAME => range.all = true,
                NO_OOV_BOW_NAME => range.flags |= NO_OOV_BOW,
                NO_OOV_EOW_NAME => range.flags |= NO_OOV_EOW,
                _ => range
                    .categories
                    .push(self.defined_category_id(source, line_number, name)?),
            }
        }
        self.sudachi_ranges.push(range);
        Ok(())
    }

    /// Parses a category line (`NAME INVOKE GROUP LENGTH`).
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the `char.def` content, for the messages.
    /// * `line_number` - The 1-based number of the line.
    /// * `line` - The line without its comment.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error when the line does not have four fields, a
    /// number does not parse, or the category is already defined. In the
    /// Sudachi format, also when the name is reserved (`ALL`, `NOOOVBOW`,
    /// `NOOOVEOW`) or the category would be one too many.
    fn parse_category(
        &mut self,
        source: &str,
        line_number: usize,
        line: &str,
    ) -> LinderaResult<()> {
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
        // A second definition would push its data into the slot of the next
        // category, shifting every later category's data, so it is an error,
        // as in MeCab.
        let name = fields[0];
        if self.category_index.contains_key(name) {
            let defined_on = match self.category_lines.get(name) {
                Some(defined_on) => format!(" on line {defined_on}"),
                None => String::new(),
            };
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: line {line_number}: category {name} is already defined{defined_on}"
            )));
        }
        if self.format == CharDefFormat::Sudachi {
            if SUDACHI_RESERVED_NAMES.contains(&name) {
                return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "{source}: line {line_number}: {name} is reserved in Sudachi's char.def \
                     format and cannot be defined as a category"
                )));
            }
            if self.category_definition.len() == SUDACHI_MAX_CATEGORIES {
                return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "{source}: line {line_number}: Sudachi's char.def format takes at most \
                     {SUDACHI_MAX_CATEGORIES} categories"
                )));
            }
        }
        // force a category_id allocation
        self.category_id(name);
        self.category_definition.push(category_data);
        self.category_lines.insert(name.to_string(), line_number);

        Ok(())
    }

    /// Parses `char.def` content line by line.
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the content, such as the path of the file,
    ///   for the messages.
    /// * `content` - The text of a `char.def` file.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or the error of the first line that cannot be parsed. In
    /// the Sudachi format, also an error when no line defines `DEFAULT`,
    /// which the lattice falls back to (#1388).
    fn parse(&mut self, source: &str, content: &str) -> LinderaResult<()> {
        for (index, line) in content.lines().enumerate() {
            let line_number = index + 1;
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
                match self.format {
                    CharDefFormat::Mecab => self.parse_range(source, line_number, line_str)?,
                    CharDefFormat::Sudachi => {
                        self.parse_range_sudachi(source, line_number, line_str)?
                    }
                }
            } else {
                self.parse_category(source, line_number, line_str)?;
            }
        }
        if self.format == CharDefFormat::Sudachi
            && !self.category_lines.contains_key(DEFAULT_CATEGORY_NAME)
        {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "{source}: no line defines {DEFAULT_CATEGORY_NAME}, which Sudachi's char.def \
                 format requires"
            )));
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

    /// Writes the categories of a code point in the Sudachi format into
    /// `categories_buffer`, and returns its flags (#1388).
    ///
    /// As Sudachi's `CharacterCategory::compile`, the code point gets the
    /// union of the categories and flags of every line that covers it, `ALL`
    /// standing for every category the file defines. A code point that no
    /// line gives a category or a flag is `DEFAULT`; one with flags only has
    /// no category.
    ///
    /// # Arguments
    ///
    /// * `c` - The code point to look up.
    /// * `categories_buffer` - Cleared, then filled with the category ids in
    ///   category id order.
    ///
    /// # Returns
    ///
    /// The flags ([`NO_OOV_BOW`], [`NO_OOV_EOW`]).
    fn lookup_sudachi(&self, c: u32, categories_buffer: &mut Vec<CategoryId>) -> u8 {
        categories_buffer.clear();
        let mut all = false;
        let mut flags = 0;
        for range in self
            .sudachi_ranges
            .iter()
            .filter(|range| range.low <= c && c <= range.high)
        {
            all |= range.all;
            flags |= range.flags;
            categories_buffer.extend_from_slice(&range.categories);
        }
        if all {
            categories_buffer.clear();
            categories_buffer.extend((0..self.category_definition.len()).map(CategoryId));
        } else {
            categories_buffer.sort_unstable();
            categories_buffer.dedup();
        }
        if categories_buffer.is_empty()
            && flags == 0
            && let Some(default_category) = self.default_category_id()
        {
            categories_buffer.push(default_category);
        }
        flags
    }

    /// Writes the default category of a code point into `categories_buffer`.
    ///
    /// In the MeCab format, the default category is the first category of
    /// the last line that covers the code point (MeCab's `default_type`), or
    /// `DEFAULT` when no line covers it. The Sudachi format has no default
    /// category; it gives the category with the lowest id.
    ///
    /// # Arguments
    ///
    /// * `c` - The code point to look up.
    /// * `categories_buffer` - Cleared, then filled with at most one category id.
    fn lookup_default_category(&self, c: u32, categories_buffer: &mut Vec<CategoryId>) {
        if self.format == CharDefFormat::Sudachi {
            self.lookup_sudachi(c, categories_buffer);
            categories_buffer.truncate(1);
            return;
        }
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
        let boundaries_set: BTreeSet<u32> = match self.format {
            CharDefFormat::Mecab => self
                .char_ranges
                .iter()
                .flat_map(|(low, high, _)| [*low, *high + 1u32])
                .collect(),
            CharDefFormat::Sudachi => self
                .sudachi_ranges
                .iter()
                .flat_map(|range| [range.low, range.high + 1u32])
                .collect(),
        };
        boundaries_set.into_iter().collect()
    }

    /// Builds the table that maps every code point to its categories.
    ///
    /// # Returns
    ///
    /// The lookup table stored in `char_def.bin`.
    fn build_lookup_table(&self) -> LookupTable<CategoryId> {
        match self.format {
            CharDefFormat::Mecab => LookupTable::from_fn(self.range_boundaries(), &|c, buff| {
                self.lookup_categories(c, buff)
            }),
            CharDefFormat::Sudachi => LookupTable::from_fn(self.range_boundaries(), &|c, buff| {
                self.lookup_sudachi(c, buff);
            }),
        }
    }

    /// Builds a table that maps every code point to its default category.
    ///
    /// In the MeCab format, the default category is the first category of
    /// the last `char.def` line that covers the code point (MeCab's
    /// `default_type`), or `DEFAULT` when no line covers it. The Sudachi
    /// format has no default category; the table gives the category with the
    /// lowest id. The dictionary itself does not store it; the trainer, which
    /// reads the MeCab format only, uses it for the `%t` feature. Call it
    /// after [`Self::build_from_str`] or [`Self::build`].
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
        match self.format {
            CharDefFormat::Mecab => {
                CharacterDefinition::new(self.category_definition.clone(), category_names, mapping)
            }
            CharDefFormat::Sudachi => {
                let mut buffer = Vec::new();
                let flags = mapping
                    .boundaries()
                    .iter()
                    .map(|&boundary| self.lookup_sudachi(boundary, &mut buffer))
                    .collect();
                CharacterDefinition::new_sudachi(
                    self.category_definition.clone(),
                    category_names,
                    mapping,
                    flags,
                )
            }
        }
    }

    /// Parses `char.def` content and returns the character definition it
    /// describes.
    ///
    /// Unlike [`Self::build`], it reads no file and writes no `char_def.bin`,
    /// so other components (such as the trainer) can read `char.def` with
    /// exactly the rules the dictionary builder uses. It reads the content in
    /// the format of the options ([`CharacterDefinitionBuilderOptions::format`]):
    /// MeCab's by default, or Sudachi's (#1388).
    ///
    /// # Arguments
    ///
    /// * `content` - The text of a `char.def` file.
    ///
    /// # Returns
    ///
    /// The character definition, or an error if a line cannot be parsed. As
    /// in `mecab-dict-index`, a range line that names a category not defined
    /// on an earlier line and a category defined twice are errors, which
    /// name the line (`char.def: line N: ...`) and the category. The Sudachi
    /// format also rejects a definition of `ALL`, `NOOOVBOW` or `NOOOVEOW`,
    /// more than 64 categories, and a file without `DEFAULT`.
    pub fn build_from_str(&mut self, content: &str) -> LinderaResult<CharacterDefinition> {
        self.build_named("char.def", content)
    }

    /// [`Self::build_from_str`] with the name that its error messages give
    /// the content, so that [`Self::build`] can name the file it read.
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the content, such as the path of the file.
    /// * `content` - The text of a `char.def` file.
    ///
    /// # Returns
    ///
    /// The character definition, or an error naming `source` and the line.
    fn build_named(&mut self, source: &str, content: &str) -> LinderaResult<CharacterDefinition> {
        self.parse(source, content)?;
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
    /// cannot be read or parsed (see [`Self::build_from_str`]; the errors of
    /// its checks name the file's path), or the output cannot be written.
    pub fn build(
        &mut self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {char_def_path:?}");
        let char_def = read_file_with_encoding(&char_def_path, &self.encoding)?;

        let char_definitions = self.build_named(&char_def_path.display().to_string(), &char_def)?;

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

    /// Parses `content`, which must be rejected, and returns the message.
    fn build_error(content: &str) -> String {
        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        match builder.build_from_str(content) {
            Ok(_) => panic!("char.def content was accepted: {content:?}"),
            Err(err) => err.to_string(),
        }
    }

    /// #1195: a range line that names a category no line defines used to
    /// give the category an id without data, and looking the category up
    /// panicked at tokenization.
    #[test]
    fn test_range_line_with_undefined_category_is_an_error() {
        let message =
            build_error("DEFAULT 0 1 0\nHIRAGANA 1 1 0\n0x3041..0x3096 HIRAGANA\n0x3042 FOO\n");

        assert!(
            message.contains(
                "char.def: line 4: category FOO is not defined; a range line can only name a \
                 category defined on an earlier line"
            ),
            "{message}"
        );
    }

    /// #1195: a range line before the definition of its category used to
    /// take the category's id, so the categories defined in between got the
    /// data of the next definition. MeCab checks a range line against the
    /// categories defined so far.
    #[test]
    fn test_range_line_before_the_category_definition_is_an_error() {
        let message =
            build_error("DEFAULT 0 1 0\n0x3041..0x3096 HIRAGANA\nKANJI 0 0 2\nHIRAGANA 1 1 0\n");

        assert!(
            message.contains("char.def: line 2: category HIRAGANA is not defined"),
            "{message}"
        );
    }

    /// #1195: a second definition of a category used to push its data into
    /// the slot of the next category.
    #[test]
    fn test_category_defined_twice_is_an_error() {
        let message =
            build_error("DEFAULT 0 1 0\nKANJI 0 0 2\n# KANJI again\nKANJI 1 1 0\nHIRAGANA 1 1 0\n");

        assert!(
            message.contains("char.def: line 4: category KANJI is already defined on line 2"),
            "{message}"
        );
    }

    /// `build` names the file it read in the messages of the checks.
    #[test]
    fn test_build_names_the_file() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            input_dir.path().join("char.def"),
            "DEFAULT 0 1 0\n0x3042 FOO\n",
        )
        .unwrap();

        let mut builder = CharacterDefinitionBuilderOptions::default().builder();
        let message = builder
            .build(input_dir.path(), output_dir.path())
            .err()
            .expect("an undefined category is rejected")
            .to_string();

        let expected = format!(
            "{}: line 2: category FOO is not defined",
            input_dir.path().join("char.def").display()
        );
        assert!(message.contains(&expected), "{message}");
    }

    /// Lines of Sudachi's `char.def` (sudachi.rs v0.7.0, #1388): they
    /// overlap, use `ALL` and the flags, and go beyond the BMP. The last line
    /// gives U+E000 a flag and no category, which Sudachi's file does not.
    const SUDACHI_CHAR_DEF: &str = "\
DEFAULT         0 1 0  # DEFAULT is a mandatory category!
SPACE           0 1 0
KANJI           0 0 2
SYMBOL          1 1 0
HIRAGANA        0 1 2
KATAKANA        1 1 2
KANJINUMERIC    0 1 0  #change INVOKE 1->0

0x0020 SPACE
0x3041..0x309F  HIRAGANA
0x30A1..0x30FA  KATAKANA
0x30FC..0x30FF  KATAKANA
0x30A1          NOOOVBOW # Small A
0x30FC..0x30FE  NOOOVBOW # 'ー' 'ヽ' 'ヾ'
0x3005          KANJI NOOOVBOW
0x3007          KANJI
0x4E00..0x9FFF  KANJI
0x4E00 KANJINUMERIC KANJI
0x2000..0x206F  SYMBOL # General Punctuation
0x3000..0x303F  SYMBOL # CJK Symbol and Punctuation
0x3007 SYMBOL KANJINUMERIC
0x309b..0x309c HIRAGANA KATAKANA # voiced/semi-voiced sound marks
0x0300..0x036F ALL NOOOVBOW # Combining Diacritical Marks
0x1F3FB..0x1F3FE ALL NOOOVBOW # emoji skin tone modifiers
0x200C..0x200D ALL NOOOVBOW NOOOVEOW # Zero Width Non-Joiner/Joiner
0xE000 NOOOVBOW
";

    /// Every category of [`SUDACHI_CHAR_DEF`], in id order.
    const SUDACHI_ALL: [&str; 7] = [
        "DEFAULT",
        "SPACE",
        "KANJI",
        "SYMBOL",
        "HIRAGANA",
        "KATAKANA",
        "KANJINUMERIC",
    ];

    fn build_sudachi(content: &str) -> (CharacterDefinitionBuilder, CharacterDefinition) {
        let mut builder = CharacterDefinitionBuilderOptions::default()
            .format(CharDefFormat::Sudachi)
            .builder();
        let char_def = builder.build_from_str(content).unwrap();
        (builder, char_def)
    }

    /// Parses `content` in the Sudachi format, which must be rejected, and
    /// returns the message.
    fn build_sudachi_error(content: &str) -> String {
        let mut builder = CharacterDefinitionBuilderOptions::default()
            .format(CharDefFormat::Sudachi)
            .builder();
        match builder.build_from_str(content) {
            Ok(_) => panic!("char.def content was accepted: {content:?}"),
            Err(err) => err.to_string(),
        }
    }

    /// #1388: in the Sudachi format, a code point gets the union of the
    /// lines that cover it, in category id order, where the MeCab format
    /// lets the last line decide.
    #[test]
    fn test_sudachi_union_of_overlapping_lines() {
        let (_, char_def) = build_sudachi(SUDACHI_CHAR_DEF);

        assert_eq!(char_def.format(), CharDefFormat::Sudachi);
        // `々` and `〇` keep KANJI, which the later SYMBOL range replaces in
        // the MeCab format.
        assert_eq!(category_names(&char_def, '々'), vec!["KANJI", "SYMBOL"]);
        assert_eq!(
            category_names(&char_def, '〇'),
            vec!["KANJI", "SYMBOL", "KANJINUMERIC"]
        );
        // Category id order, not the order of the line.
        assert_eq!(
            category_names(&char_def, '一'),
            vec!["KANJI", "KANJINUMERIC"]
        );
        assert_eq!(
            category_names(&char_def, '゛'),
            vec!["HIRAGANA", "KATAKANA"]
        );
        assert_eq!(category_names(&char_def, '「'), vec!["SYMBOL"]);
        assert_eq!(category_names(&char_def, 'ア'), vec!["KATAKANA"]);
        // No line covers `・` or `a`.
        assert_eq!(category_names(&char_def, '・'), vec!["DEFAULT"]);
        assert_eq!(category_names(&char_def, 'a'), vec!["DEFAULT"]);
    }

    /// #1388: the flags are the union of the lines too, and `ALL` stands
    /// for every category the file defines.
    #[test]
    fn test_sudachi_all_and_flags() {
        let (_, char_def) = build_sudachi(SUDACHI_CHAR_DEF);

        for (c, flags) in [
            ('々', NO_OOV_BOW),
            ('ァ', NO_OOV_BOW),
            ('ー', NO_OOV_BOW),
            ('ヾ', NO_OOV_BOW),
            ('ア', 0),
            ('ヿ', 0),
            ('一', 0),
            ('\u{0301}', NO_OOV_BOW),
            ('\u{200D}', NO_OOV_BOW | NO_OOV_EOW),
            ('\u{2010}', 0),
        ] {
            assert_eq!(char_def.flags_of(c), flags, "U+{:04X}", c as u32);
        }
        assert_eq!(category_names(&char_def, '\u{0301}'), SUDACHI_ALL);
        assert_eq!(category_names(&char_def, '\u{200D}'), SUDACHI_ALL);
        assert_eq!(category_names(&char_def, '\u{2010}'), vec!["SYMBOL"]);
    }

    /// #1388: the Sudachi format reads code points beyond the BMP.
    #[test]
    fn test_sudachi_code_points_beyond_the_bmp() {
        let (_, char_def) = build_sudachi(SUDACHI_CHAR_DEF);

        assert_eq!(category_names(&char_def, '\u{1F3FB}'), SUDACHI_ALL);
        assert_eq!(char_def.flags_of('\u{1F3FE}'), NO_OOV_BOW);
        assert_eq!(category_names(&char_def, '\u{1F3FF}'), vec!["DEFAULT"]);
        assert_eq!(char_def.flags_of('\u{1F3FF}'), 0);
    }

    /// #1388: a code point with flags and no category has no category, as
    /// in Sudachi's `CharacterCategory::compile`, not `DEFAULT`.
    #[test]
    fn test_sudachi_flags_without_category() {
        let (_, char_def) = build_sudachi(SUDACHI_CHAR_DEF);

        assert!(char_def.lookup_categories('\u{E000}').is_empty());
        assert_eq!(char_def.flags_of('\u{E000}'), NO_OOV_BOW);
    }

    /// #1388: the Sudachi format still rejects a name that no earlier line
    /// defines, without the hint of the MeCab format.
    #[test]
    fn test_sudachi_rejects_an_undefined_category() {
        let message = build_sudachi_error("DEFAULT 0 1 0\nHIRAGANA 1 1 0\n0x3042 FOO\n");

        assert!(
            message.contains("char.def: line 3: category FOO is not defined"),
            "{message}"
        );
        assert!(!message.contains("char_def_format"), "{message}");
    }

    /// #1388: the names that only a range line of the Sudachi format reads
    /// cannot be defined as categories.
    #[test]
    fn test_sudachi_rejects_a_reserved_name_as_a_category() {
        for name in ["ALL", "NOOOVBOW", "NOOOVEOW"] {
            let message = build_sudachi_error(&format!("DEFAULT 0 1 0\n{name} 0 1 0\n"));

            assert!(
                message.contains(&format!(
                    "char.def: line 2: {name} is reserved in Sudachi's char.def format"
                )),
                "{message}"
            );
        }
    }

    /// #1388: the Sudachi format needs `DEFAULT`, which the lattice falls
    /// back to.
    #[test]
    fn test_sudachi_requires_default() {
        let message = build_sudachi_error("KANJI 0 0 2\n0x4E00..0x9FFF KANJI\n");

        assert!(
            message.contains("char.def: no line defines DEFAULT"),
            "{message}"
        );
    }

    /// #1388: the Sudachi format takes 64 categories, not 65.
    #[test]
    fn test_sudachi_category_limit() {
        let definitions = |count: usize| {
            std::iter::once("DEFAULT 0 1 0\n".to_string())
                .chain((1..count).map(|i| format!("C{i} 0 1 0\n")))
                .collect::<String>()
        };
        let (_, char_def) = build_sudachi(&definitions(64));
        assert_eq!(char_def.categories().len(), 64);

        let message = build_sudachi_error(&definitions(65));
        assert!(
            message.contains("char.def: line 65: Sudachi's char.def format takes at most 64"),
            "{message}"
        );
    }

    /// #1388: a bound that is not a Unicode scalar value, and a range that
    /// ends before it starts, are errors.
    #[test]
    fn test_sudachi_rejects_invalid_ranges() {
        for (range, expected) in [
            ("0xD800", "0xD800 is not a code point"),
            ("0x110000", "0x110000 is not a code point"),
            ("0xZZ", "0xZZ is not a code point"),
            (
                "0x3042..0x3041",
                "the range 0x3042..0x3041 ends before it starts",
            ),
        ] {
            let message = build_sudachi_error(&format!("DEFAULT 0 1 0\n{range} DEFAULT\n"));

            assert!(
                message.contains(&format!("char.def: line 2: {expected}")),
                "{message}"
            );
        }
    }

    /// #1388: the MeCab format rejects the syntax of Sudachi's `char.def`
    /// with a hint at the format, and that the trainer reads MeCab's only.
    #[test]
    fn test_mecab_format_hints_at_the_sudachi_format() {
        for name in ["ALL", "NOOOVBOW", "NOOOVEOW"] {
            let message = build_error(&format!(
                "DEFAULT 0 1 0\nKATAKANA 1 1 2\n0x30A1..0x30FA KATAKANA\n0x30A1 {name}\n"
            ));

            assert!(
                message.contains(&format!(
                    "char.def: line 4: category {name} is not defined; a range line can only \
                     name a category defined on an earlier line ({name} is the syntax of \
                     Sudachi's char.def, which a dictionary selects with \"char_def_format\": \
                     \"sudachi\" in its metadata.json; lindera dictionary train reads MeCab's format only)"
                )),
                "{message}"
            );
        }
        let message = build_error("DEFAULT 0 1 0\n0x3042 FOO\n");
        assert!(!message.contains("char_def_format"), "{message}");
    }

    /// #1388: `build` writes the format, the categories and the flags of
    /// the Sudachi format into `char_def.bin`.
    #[test]
    fn test_build_writes_the_same_sudachi_definition() {
        let input_dir = tempfile::tempdir().unwrap();
        let output_dir = tempfile::tempdir().unwrap();
        std::fs::write(input_dir.path().join("char.def"), SUDACHI_CHAR_DEF).unwrap();

        let mut builder = CharacterDefinitionBuilderOptions::default()
            .format(CharDefFormat::Sudachi)
            .builder();
        builder.build(input_dir.path(), output_dir.path()).unwrap();
        let written = std::fs::read(output_dir.path().join("char_def.bin")).unwrap();
        let loaded = CharacterDefinition::load(&written).unwrap();

        let (_, expected) = build_sudachi(SUDACHI_CHAR_DEF);
        assert_eq!(loaded.format(), CharDefFormat::Sudachi);
        for c in [
            '々',
            '〇',
            '一',
            'ァ',
            '・',
            'a',
            '\u{0301}',
            '\u{200D}',
            '\u{1F3FB}',
            '\u{E000}',
        ] {
            assert_eq!(category_names(&loaded, c), category_names(&expected, c));
            assert_eq!(loaded.flags_of(c), expected.flags_of(c));
        }
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

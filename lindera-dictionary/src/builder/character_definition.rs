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

    fn lookup_categories(&self, c: u32, categories_buffer: &mut Vec<CategoryId>) {
        categories_buffer.clear();
        for (start, stop, category_ids) in &self.char_ranges {
            if *start <= c && *stop >= c {
                for cat in category_ids {
                    if !categories_buffer.contains(cat) {
                        categories_buffer.push(*cat);
                    }
                }
            }
        }
        if categories_buffer.is_empty()
            && let Some(default_category) = self.category_index.get(DEFAULT_CATEGORY_NAME)
        {
            categories_buffer.push(*default_category);
        }
    }

    fn build_lookup_table(&self) -> LookupTable<CategoryId> {
        let boundaries_set: BTreeSet<u32> = self
            .char_ranges
            .iter()
            .flat_map(|(low, high, _)| vec![*low, *high + 1u32])
            .collect();
        let boundaries: Vec<u32> = boundaries_set.into_iter().collect();
        LookupTable::from_fn(boundaries, &|c, buff| self.lookup_categories(c, buff))
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

    pub fn build(
        &mut self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {char_def_path:?}");
        let char_def = read_file_with_encoding(&char_def_path, &self.encoding)?;

        // let mut char_definitions_builder = CharacterDefinitionsBuilder::default();
        self.parse(&char_def)?;
        let char_definitions = self.get_character_definition().clone();

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

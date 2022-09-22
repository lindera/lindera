use std::collections::{BTreeSet, HashMap};

use byteorder::{ByteOrder, LittleEndian};
use encoding_rs::UTF_16LE;
use serde::{Deserialize, Serialize};

use crate::error::LinderaErrorKind;
use crate::LinderaResult;

const DEFAULT_CATEGORY_NAME: &str = "DEFAULT";

fn ucs2_to_unicode(ucs2_codepoint: u16) -> LinderaResult<u32> {
    let mut buf = [0u8; 2];
    LittleEndian::write_u16(&mut buf[..], ucs2_codepoint);

    let s = UTF_16LE.decode(&buf[..]).0.into_owned();
    let chrs: Vec<char> = s.chars().collect();

    match chrs.len() {
        1 => Ok(chrs[0] as u32),
        _ => Err(LinderaErrorKind::Parse.with_error(anyhow::anyhow!("unusual char length"))),
    }
}

fn parse_hex_codepoint(s: &str) -> LinderaResult<u32> {
    let removed_0x = s.trim_start_matches("0x");
    let ucs2_codepoint = u16::from_str_radix(removed_0x, 16)
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;

    ucs2_to_unicode(ucs2_codepoint)
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct CategoryData {
    pub invoke: bool,
    pub group: bool,
    pub length: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Copy, PartialOrd, Ord, Eq, PartialEq)]
pub struct CategoryId(pub usize);

#[derive(Clone, Serialize, Deserialize)]
pub struct CharacterDefinitions {
    pub category_definitions: Vec<CategoryData>,
    pub category_names: Vec<String>,
    pub mapping: LookupTable<CategoryId>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LookupTable<T: Copy + Clone> {
    boundaries: Vec<u32>,
    values: Vec<Vec<T>>,
}

impl<T: Copy + Clone> LookupTable<T> {
    pub fn from_fn(mut boundaries: Vec<u32>, funct: &dyn Fn(u32, &mut Vec<T>)) -> LookupTable<T> {
        if !boundaries.contains(&0) {
            boundaries.push(0);
        }
        boundaries.sort_unstable();
        let mut values = Vec::new();
        for &boundary in &boundaries {
            let mut output = Vec::default();
            funct(boundary, &mut output);
            values.push(output);
        }
        LookupTable { boundaries, values }
    }

    pub fn eval(&self, target: u32) -> &[T] {
        let idx = self
            .boundaries
            .binary_search(&target)
            .unwrap_or_else(|val| val - 1);
        &self.values[idx][..]
    }
}

impl CharacterDefinitions {
    pub fn categories(&self) -> &[String] {
        &self.category_names[..]
    }

    pub fn load(char_def_data: &[u8]) -> LinderaResult<CharacterDefinitions> {
        bincode::deserialize(char_def_data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err)))
    }

    pub fn lookup_definition(&self, category_id: CategoryId) -> &CategoryData {
        &self.category_definitions[category_id.0]
    }

    pub fn category_name(&self, category_id: CategoryId) -> &str {
        &self.category_names[category_id.0 as usize]
    }

    pub fn lookup_categories(&self, c: char) -> &[CategoryId] {
        self.mapping.eval(c as u32)
    }
}

#[derive(Default)]
pub struct CharacterDefinitionsBuilder {
    category_definition: Vec<CategoryData>,
    category_index: HashMap<String, CategoryId>,
    char_ranges: Vec<(u32, u32, Vec<CategoryId>)>,
}

impl CharacterDefinitionsBuilder {
    pub fn category_id(&mut self, category_name: &str) -> CategoryId {
        let num_categories = self.category_index.len();
        *self
            .category_index
            .entry(category_name.to_string())
            .or_insert(CategoryId(num_categories))
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
        if categories_buffer.is_empty() {
            if let Some(default_category) = self.category_index.get(DEFAULT_CATEGORY_NAME) {
                categories_buffer.push(*default_category);
            }
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

    pub fn parse(&mut self, content: &str) -> LinderaResult<()> {
        for line in content.lines() {
            let line_str = line
                .split('#')
                .next()
                .ok_or_else(|| {
                    LinderaErrorKind::Parse.with_error(anyhow::anyhow!("failed to parse line"))
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
                return Err(
                    LinderaErrorKind::Content.with_error(anyhow::anyhow!("Invalid line: {}", line))
                );
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
            )));
        }
        let invoke = fields[1]
            .parse::<u32>()
            .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?
            == 1;
        let group = fields[2]
            .parse::<u32>()
            .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?
            == 1;
        let length = fields[3]
            .parse::<u32>()
            .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
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

    pub fn build(self) -> CharacterDefinitions {
        let mut category_names: Vec<String> = (0..self.category_index.len())
            .map(|_| String::new())
            .collect();
        for (category_name, category_id) in &self.category_index {
            category_names[category_id.0] = category_name.clone();
        }
        let mapping = self.build_lookup_table();
        CharacterDefinitions {
            category_definitions: self.category_definition,
            category_names,
            mapping,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::character_definition::LookupTable;

    #[test]
    fn test_lookup_table() {
        let funct = |c: u32, output: &mut Vec<u32>| {
            if c >= 10u32 {
                output.push(1u32);
            } else {
                output.push(0u32);
            }
        };
        let lookup_table = LookupTable::from_fn(vec![0u32, 10u32], &funct);
        for i in 0..100 {
            let mut v = Vec::default();
            funct(i, &mut v);
            assert_eq!(lookup_table.eval(i), &v[..]);
        }
    }

    //    #[test]
    //    fn test_bisa() {
    //        let char_definitions = CharacterDefinitions::load();
    //        let category_ids: Vec<&str> = char_definitions
    //            .lookup_categories('々')
    //            .iter()
    //            .map(|&category_id| char_definitions.category_name(category_id))
    //            .collect();
    //        assert_eq!(category_ids, &["KANJI", "SYMBOL"]);
    //    }

    //    #[test]
    //    fn test_jp_hyphen() {
    //        let char_definitions = CharacterDefinitions::load();
    //        let category_ids: Vec<&str> = char_definitions
    //            .lookup_categories('ー')
    //            .iter()
    //            .map(|&category_id| char_definitions.category_name(category_id))
    //            .collect();
    //        assert_eq!(category_ids, &["KATAKANA"]);
    //    }

    //    #[test]
    //    fn test_char_definitions() {
    //        let char_definitions = CharacterDefinitions::load();
    //        {
    //            let v = char_definitions.lookup_categories('あ');
    //            assert_eq!(v.len(), 1);
    //            assert_eq!(char_definitions.category_name(v[0]), "HIRAGANA");
    //        }
    //        {
    //            let v = char_definitions.lookup_categories('@');
    //            assert_eq!(v.len(), 1);
    //            assert_eq!(char_definitions.category_name(v[0]), "SYMBOL");
    //        }
    //        {
    //            let v = char_definitions.lookup_categories('一');
    //            assert_eq!(v.len(), 2);
    //            assert_eq!(char_definitions.category_name(v[0]), "KANJI");
    //            assert_eq!(char_definitions.category_name(v[1]), "KANJINUMERIC");
    //        }
    //    }
}

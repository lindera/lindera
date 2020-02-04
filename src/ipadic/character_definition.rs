use lindera_ipadic::char_def;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct CategoryData {
    pub invoke: bool,
    pub group: bool,
    pub length: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Copy, PartialOrd, Ord, Eq, PartialEq)]
pub struct CategoryId(pub usize);

#[derive(Serialize, Deserialize)]
pub struct CharacterDefinitions {
    pub category_definitions: Vec<CategoryData>,
    pub category_names: Vec<String>,
    pub mapping: LookupTable<CategoryId>,
}

#[derive(Serialize, Deserialize)]
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

    pub fn load() -> CharacterDefinitions {
        bincode::deserialize(char_def()).expect("Failed to deserialize char definition data")
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

#[cfg(test)]
mod tests {
    use crate::ipadic::character_definition::{CharacterDefinitions, LookupTable};

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

    #[test]
    fn test_bisa() {
        let char_definitions = CharacterDefinitions::load();
        let category_ids: Vec<&str> = char_definitions
            .lookup_categories('々')
            .iter()
            .map(|&category_id| char_definitions.category_name(category_id))
            .collect();
        assert_eq!(category_ids, &["KANJI", "SYMBOL"]);
    }

    #[test]
    fn test_jp_hyphen() {
        let char_definitions = CharacterDefinitions::load();
        let category_ids: Vec<&str> = char_definitions
            .lookup_categories('ー')
            .iter()
            .map(|&category_id| char_definitions.category_name(category_id))
            .collect();
        assert_eq!(category_ids, &["KATAKANA"]);
    }

    #[test]
    fn test_char_definitions() {
        let char_definitions = CharacterDefinitions::load();
        {
            let v = char_definitions.lookup_categories('あ');
            assert_eq!(v.len(), 1);
            assert_eq!(char_definitions.category_name(v[0]), "HIRAGANA");
        }
        {
            let v = char_definitions.lookup_categories('@');
            assert_eq!(v.len(), 1);
            assert_eq!(char_definitions.category_name(v[0]), "SYMBOL");
        }
        {
            let v = char_definitions.lookup_categories('一');
            assert_eq!(v.len(), 2);
            assert_eq!(char_definitions.category_name(v[0]), "KANJI");
            assert_eq!(char_definitions.category_name(v[1]), "KANJINUMERIC");
        }
    }
}

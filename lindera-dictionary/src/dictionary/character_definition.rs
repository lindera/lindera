use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::error::LinderaErrorKind;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct CategoryData {
    pub invoke: bool,
    pub group: bool,
    pub length: u32,
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Copy,
    PartialOrd,
    Ord,
    Eq,
    PartialEq,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]

pub struct CategoryId(pub usize);

#[derive(Serialize, Deserialize, Clone, Archive, RkyvSerialize, RkyvDeserialize)]

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

impl<T: Copy + Clone + Archive> ArchivedLookupTable<T> {
    pub fn eval(&self, target: u32) -> &[T::Archived] {
        let target_le = rkyv::rend::u32_le::from_native(target);
        let idx = self
            .boundaries
            .binary_search(&target_le)
            .unwrap_or_else(|val| val - 1);
        self.values[idx].as_slice()
    }
}

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct CharacterDefinition {
    pub category_definitions: Vec<CategoryData>,
    pub category_names: Vec<String>,
    pub mapping: LookupTable<CategoryId>,
}

impl CharacterDefinition {
    pub fn categories(&self) -> &[String] {
        &self.category_names[..]
    }

    pub fn load(char_def_data: &[u8]) -> LinderaResult<CharacterDefinition> {
        let mut aligned = rkyv::util::AlignedVec::<16>::new();
        aligned.extend_from_slice(char_def_data);
        rkyv::from_bytes::<CharacterDefinition, rkyv::rancor::Error>(&aligned).map_err(|err| {
            LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
        })
    }

    pub fn lookup_definition(&self, category_id: CategoryId) -> &CategoryData {
        &self.category_definitions[category_id.0]
    }

    pub fn category_name(&self, category_id: CategoryId) -> &str {
        &self.category_names[category_id.0]
    }

    pub fn category_id_by_name(&self, name: &str) -> Option<CategoryId> {
        self.category_names
            .iter()
            .position(|n| n == name)
            .map(CategoryId)
    }

    pub fn lookup_categories(&self, c: char) -> &[CategoryId] {
        self.mapping.eval(c as u32)
    }
}

impl ArchivedCharacterDefinition {
    pub fn categories(&self) -> &[rkyv::string::ArchivedString] {
        &self.category_names[..]
    }

    pub fn lookup_definition(&self, category_id: usize) -> &ArchivedCategoryData {
        &self.category_definitions[category_id]
    }

    pub fn category_name(&self, category_id: usize) -> &str {
        self.category_names[category_id].as_str()
    }

    pub fn lookup_categories(&self, c: char) -> &[ArchivedCategoryId] {
        self.mapping.eval(c as u32)
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary::character_definition::LookupTable;

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
}

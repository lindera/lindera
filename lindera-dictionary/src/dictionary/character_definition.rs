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

/// Number of codepoints covered by the flat category table (the Basic
/// Multilingual Plane).
const FLAT_TABLE_LEN: usize = 0x10000;

/// Maximum categories per codepoint representable in a packed `flat_index`
/// entry (8 bits of length).
const FLAT_MAX_ROW_LEN: usize = 0xFF;

/// Maximum pool offset representable in a packed `flat_index` entry (24 bits).
const FLAT_MAX_OFFSET: usize = (1 << 24) - 1;

#[derive(Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]

pub struct CharacterDefinition {
    pub category_definitions: Vec<CategoryData>,
    pub category_names: Vec<String>,
    pub mapping: LookupTable<CategoryId>,
    /// Concatenation of the `mapping` rows in row order; the backing pool
    /// sliced by `flat_index`. Runtime-only: skipped by serde and rkyv (a
    /// trailing zero-sized member in the archived layout), rebuilt at load.
    #[serde(skip)]
    #[rkyv(with = ::rkyv::with::Skip)]
    flat_categories: Vec<CategoryId>,
    /// Per-BMP-codepoint packed `offset << 8 | len` into `flat_categories`,
    /// `FLAT_TABLE_LEN` entries; empty when the table is not built, in which
    /// case lookups fall back to the `mapping` binary search. Runtime-only:
    /// skipped by serde and rkyv, rebuilt at load.
    #[serde(skip)]
    #[rkyv(with = ::rkyv::with::Skip)]
    flat_index: Vec<u32>,
}

impl CharacterDefinition {
    /// Creates a definition from its parsed parts and builds the flat
    /// BMP category table.
    ///
    /// # Arguments
    ///
    /// * `category_definitions` - Per-category invoke/group/length flags.
    /// * `category_names` - Category names in `CategoryId` order.
    /// * `mapping` - Codepoint-range to category-set lookup table.
    ///
    /// # Returns
    ///
    /// A `CharacterDefinition` ready for O(1) BMP category lookups.
    pub fn new(
        category_definitions: Vec<CategoryData>,
        category_names: Vec<String>,
        mapping: LookupTable<CategoryId>,
    ) -> Self {
        let mut definition = CharacterDefinition {
            category_definitions,
            category_names,
            mapping,
            flat_categories: Vec::new(),
            flat_index: Vec::new(),
        };
        definition.build_flat_table();
        definition
    }

    /// Builds the flat BMP category table from `mapping`.
    ///
    /// Leaves the table empty (falling back to the binary search) if a row
    /// exceeds the packed-entry limits, which no real char.def can reach.
    fn build_flat_table(&mut self) {
        let mut pool: Vec<CategoryId> = Vec::new();
        // One packed (offset, len) per mapping row, in row order.
        let mut packed_rows: Vec<u32> = Vec::with_capacity(self.mapping.values.len());
        for row in &self.mapping.values {
            let offset = pool.len();
            if row.len() > FLAT_MAX_ROW_LEN || offset > FLAT_MAX_OFFSET {
                return;
            }
            pool.extend_from_slice(row);
            packed_rows.push(((offset as u32) << 8) | row.len() as u32);
        }

        let mut index = Vec::with_capacity(FLAT_TABLE_LEN);
        for cp in 0..FLAT_TABLE_LEN as u32 {
            let row_idx = self
                .mapping
                .boundaries
                .binary_search(&cp)
                .unwrap_or_else(|val| val - 1);
            index.push(packed_rows[row_idx]);
        }

        self.flat_categories = pool;
        self.flat_index = index;
    }

    pub fn categories(&self) -> &[String] {
        &self.category_names[..]
    }

    pub fn load(char_def_data: &[u8]) -> LinderaResult<CharacterDefinition> {
        let mut aligned = rkyv::util::AlignedVec::<16>::new();
        aligned.extend_from_slice(char_def_data);
        let mut definition = rkyv::from_bytes::<CharacterDefinition, rkyv::rancor::Error>(&aligned)
            .map_err(|err| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(err.to_string()))
            })?;
        // The flat table is not serialized; rebuild it after deserialization.
        definition.build_flat_table();
        Ok(definition)
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
        let cp = c as u32;
        if (cp as usize) < FLAT_TABLE_LEN && !self.flat_index.is_empty() {
            // O(1) fast path: one indexed load plus a slice into the pool,
            // returning exactly the slice the binary search would.
            let packed = self.flat_index[cp as usize];
            let offset = (packed >> 8) as usize;
            let len = (packed & 0xFF) as usize;
            &self.flat_categories[offset..offset + len]
        } else {
            self.mapping.eval(cp)
        }
    }

    /// Returns the pool coordinates of `c`'s category set when the flat BMP
    /// fast path can serve it, so callers can store the compact
    /// `(offset, len)` pair instead of copying the categories (#942).
    ///
    /// # 引数
    ///
    /// * `c` - The character to look up.
    ///
    /// # 戻り値
    ///
    /// `Some((offset, len))` addressing [`Self::flat_category`] (offset is
    /// at most 24 bits by construction), or `None` when the flat table is
    /// unavailable or `c` is outside the BMP.
    #[inline(always)]
    pub(crate) fn lookup_categories_packed(&self, c: char) -> Option<(u32, u16)> {
        let cp = c as usize;
        if cp < FLAT_TABLE_LEN && !self.flat_index.is_empty() {
            let packed = self.flat_index[cp];
            Some((packed >> 8, (packed & 0xFF) as u16))
        } else {
            None
        }
    }

    /// Returns the category at `idx` in the flat pool.
    ///
    /// # 引数
    ///
    /// * `idx` - Pool index derived from [`Self::lookup_categories_packed`].
    ///
    /// # 戻り値
    ///
    /// The category id stored at that pool slot.
    #[inline(always)]
    pub(crate) fn flat_category(&self, idx: usize) -> CategoryId {
        self.flat_categories[idx]
    }
}

#[cfg(test)]
mod tests {
    use crate::dictionary::character_definition::{
        CategoryData, CategoryId, CharacterDefinition, LookupTable,
    };

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

    /// Builds a small multi-category definition whose ranges cross the
    /// codepoint space, including a multi-category row and a boundary
    /// beyond the BMP.
    fn test_definition() -> CharacterDefinition {
        let mapping = LookupTable::from_fn(
            vec![0u32, 0x80, 0x3040, 0x4E00, 0x20000],
            &|c, buff: &mut Vec<CategoryId>| {
                if c >= 0x20000 {
                    buff.push(CategoryId(3));
                } else if c >= 0x4E00 {
                    // Multi-category row: order must be preserved.
                    buff.push(CategoryId(2));
                    buff.push(CategoryId(1));
                } else if c >= 0x3040 {
                    buff.push(CategoryId(1));
                } else if c >= 0x80 {
                    buff.push(CategoryId(3));
                } else {
                    buff.push(CategoryId(0));
                }
            },
        );
        let categories = vec![
            CategoryData {
                invoke: false,
                group: true,
                length: 0,
            };
            4
        ];
        let names = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        CharacterDefinition::new(categories, names, mapping)
    }

    /// Regression test for #878 stage 2: the flat BMP table must return
    /// exactly the slice (contents and order) the binary search returns,
    /// for every codepoint including the astral fallback.
    #[test]
    fn test_flat_table_matches_binary_search_for_all_chars() {
        let definition = test_definition();
        assert!(
            !definition.flat_index.is_empty(),
            "flat table should be built"
        );
        for cp in 0..=0x10FFFFu32 {
            let Some(c) = char::from_u32(cp) else {
                continue; // surrogate range
            };
            assert_eq!(
                definition.lookup_categories(c),
                definition.mapping.eval(cp),
                "mismatch at U+{cp:04X}"
            );
        }
    }

    /// Regression test for #878 stage 2: the flat table is skipped during
    /// serialization and rebuilt by `load()`, and lookups survive the
    /// round-trip unchanged.
    #[test]
    fn test_flat_table_rebuilt_after_rkyv_round_trip() {
        let definition = test_definition();
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&definition).unwrap();
        let reloaded = CharacterDefinition::load(&bytes).unwrap();
        assert!(!reloaded.flat_index.is_empty(), "load() must rebuild");
        for cp in [0u32, 0x41, 0x80, 0x3042, 0x4E8C, 0xFFFF, 0x20B9F] {
            let c = char::from_u32(cp).unwrap();
            assert_eq!(
                reloaded.lookup_categories(c),
                definition.lookup_categories(c)
            );
        }
    }
}

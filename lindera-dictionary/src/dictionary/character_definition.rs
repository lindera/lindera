use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

use crate::LinderaResult;
use crate::error::LinderaErrorKind;

/// The format of a dictionary's `char.def`, which its `metadata.json` selects
/// with `char_def_format` (#1388). It decides how the builder reads the file
/// and how the lattice creates unknown words.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum CharDefFormat {
    /// MeCab's format, the default: the last line that covers a code point
    /// decides its categories (#1095), and the first of them, its default
    /// category, alone creates unknown words (#1111).
    #[default]
    Mecab,
    /// Sudachi's format: a code point gets the union of the categories of
    /// every line that covers it, `ALL` names every category, and the flags
    /// `NOOOVBOW` and `NOOOVEOW` keep unknown words from starting at or
    /// right after it. Every category of a character creates unknown words,
    /// over runs cut as Sudachi cuts them, and a position where nothing else
    /// starts gets a one-character unknown word of `DEFAULT`, as in Sudachi.
    Sudachi,
}

impl CharDefFormat {
    /// Returns whether this is MeCab's format, the default, which
    /// `metadata.json` leaves out.
    ///
    /// # Returns
    ///
    /// `true` for [`CharDefFormat::Mecab`].
    pub fn is_mecab(&self) -> bool {
        *self == CharDefFormat::Mecab
    }
}

/// Flag of a code point in the Sudachi format (`NOOOVBOW`): no unknown word
/// starts at it.
pub const NO_OOV_BOW: u8 = 1;

/// Flag of a code point in the Sudachi format (`NOOOVEOW`): no unknown word
/// starts right after it.
pub const NO_OOV_EOW: u8 = 2;

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

    /// Returns the index of the row that covers `target`.
    ///
    /// # Arguments
    ///
    /// * `target` - The code point to look up.
    ///
    /// # Returns
    ///
    /// The index into the rows, which [`Self::eval`] reads.
    pub(crate) fn row_index(&self, target: u32) -> usize {
        self.boundaries
            .binary_search(&target)
            .unwrap_or_else(|val| val - 1)
    }

    /// Returns the number of rows.
    ///
    /// # Returns
    ///
    /// The number of rows, one per boundary.
    pub(crate) fn row_count(&self) -> usize {
        self.values.len()
    }

    /// Returns the first code point of each row, in row order.
    ///
    /// # Returns
    ///
    /// The boundaries, sorted, the first being 0.
    pub(crate) fn boundaries(&self) -> &[u32] {
        &self.boundaries
    }

    /// Returns whether `pred` holds for the values of any range that covers
    /// a codepoint at or above `from`.
    ///
    /// # Arguments
    ///
    /// * `from` - The lowest codepoint of interest.
    /// * `pred` - The test applied to a range's values.
    ///
    /// # Returns
    ///
    /// `true` when some such range satisfies `pred`.
    pub(crate) fn any_range_from(&self, from: u32, pred: impl Fn(&[T]) -> bool) -> bool {
        self.values.iter().enumerate().any(|(i, values)| {
            // Range `i` covers `boundaries[i]..boundaries[i + 1]`; the last
            // one is open-ended.
            let end = self.boundaries.get(i + 1).copied().unwrap_or(u32::MAX);
            end > from && pred(values)
        })
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
    /// Code point ranges to their categories. In the MeCab format, each row
    /// lists the default category first, then the others in category id
    /// order (#1111); in the Sudachi format, all of them in category id
    /// order. See [`Self::lookup_categories`].
    pub mapping: LookupTable<CategoryId>,
    /// The format the builder read `char.def` in, which decides how the
    /// lattice creates unknown words (#1388).
    #[serde(default)]
    pub format: CharDefFormat,
    /// Sudachi format only: the flags ([`NO_OOV_BOW`], [`NO_OOV_EOW`]) of
    /// each `mapping` row, in row order. Empty in the MeCab format.
    #[serde(default)]
    pub flags: Vec<u8>,
    /// Sudachi format only: the flags of every BMP code point, so that the
    /// lattice reads them with one indexed load. Runtime-only like
    /// `flat_index`: skipped by serde and rkyv, rebuilt at load. Empty in the
    /// MeCab format.
    #[serde(skip)]
    #[rkyv(with = ::rkyv::with::Skip)]
    flat_flags: Vec<u8>,
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
    /// Creates a definition in the MeCab format from its parsed parts and
    /// builds the flat BMP category table.
    ///
    /// # Arguments
    ///
    /// * `category_definitions` - Per-category invoke/group/length flags.
    /// * `category_names` - Category names in `CategoryId` order.
    /// * `mapping` - Codepoint-range to category-set lookup table, each row
    ///   with its default category first.
    ///
    /// # Returns
    ///
    /// A `CharacterDefinition` ready for O(1) BMP category lookups.
    pub fn new(
        category_definitions: Vec<CategoryData>,
        category_names: Vec<String>,
        mapping: LookupTable<CategoryId>,
    ) -> Self {
        Self::with_format(
            category_definitions,
            category_names,
            mapping,
            CharDefFormat::Mecab,
            Vec::new(),
        )
    }

    /// Creates a definition in the Sudachi format (#1388) from its parsed
    /// parts and builds the flat BMP tables.
    ///
    /// # Arguments
    ///
    /// * `category_definitions` - Per-category invoke/group/length flags.
    /// * `category_names` - Category names in `CategoryId` order.
    /// * `mapping` - Codepoint-range to category-set lookup table, each row
    ///   in category id order.
    /// * `flags` - The flags ([`NO_OOV_BOW`], [`NO_OOV_EOW`]) of each row of
    ///   `mapping`, in row order.
    ///
    /// # Returns
    ///
    /// The definition.
    ///
    /// # Panics
    ///
    /// When `flags` does not have one entry per row of `mapping`.
    pub fn new_sudachi(
        category_definitions: Vec<CategoryData>,
        category_names: Vec<String>,
        mapping: LookupTable<CategoryId>,
        flags: Vec<u8>,
    ) -> Self {
        assert_eq!(
            flags.len(),
            mapping.row_count(),
            "one flag entry per row of the mapping"
        );
        Self::with_format(
            category_definitions,
            category_names,
            mapping,
            CharDefFormat::Sudachi,
            flags,
        )
    }

    /// Creates a definition and builds its runtime tables.
    ///
    /// # Arguments
    ///
    /// * `category_definitions` - Per-category invoke/group/length flags.
    /// * `category_names` - Category names in `CategoryId` order.
    /// * `mapping` - Codepoint-range to category-set lookup table.
    /// * `format` - The format of `char.def`.
    /// * `flags` - The flags of each row of `mapping`; empty in the MeCab
    ///   format.
    ///
    /// # Returns
    ///
    /// The definition.
    fn with_format(
        category_definitions: Vec<CategoryData>,
        category_names: Vec<String>,
        mapping: LookupTable<CategoryId>,
        format: CharDefFormat,
        flags: Vec<u8>,
    ) -> Self {
        let mut definition = CharacterDefinition {
            category_definitions,
            category_names,
            mapping,
            format,
            flags,
            flat_flags: Vec::new(),
            flat_categories: Vec::new(),
            flat_index: Vec::new(),
        };
        definition.build_flat_table();
        definition.build_flat_flags();
        definition
    }

    /// Builds the flat BMP flag table from `flags` (the Sudachi format), or
    /// leaves it empty when there are no flags (the MeCab format) or when a
    /// definition read from bytes does not have one entry per row.
    fn build_flat_flags(&mut self) {
        self.flat_flags.clear();
        if self.flags.is_empty() || self.flags.len() != self.mapping.row_count() {
            return;
        }
        self.flat_flags = (0..FLAT_TABLE_LEN as u32)
            .map(|cp| self.flags[self.mapping.row_index(cp)])
            .collect();
    }

    /// Returns the format the builder read `char.def` in.
    ///
    /// # Returns
    ///
    /// The format.
    #[inline]
    pub fn format(&self) -> CharDefFormat {
        self.format
    }

    /// Returns the flags of `c` in the Sudachi format ([`NO_OOV_BOW`],
    /// [`NO_OOV_EOW`]).
    ///
    /// # Arguments
    ///
    /// * `c` - The character to look up.
    ///
    /// # Returns
    ///
    /// The flags; always 0 in the MeCab format.
    #[inline]
    pub fn flags_of(&self, c: char) -> u8 {
        if self.flags.is_empty() {
            return 0;
        }
        let cp = c as u32;
        if (cp as usize) < FLAT_TABLE_LEN && !self.flat_flags.is_empty() {
            self.flat_flags[cp as usize]
        } else {
            self.flags[self.mapping.row_index(cp)]
        }
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
        // The flags must have one entry per row in the Sudachi format and
        // none in the MeCab format; `flags_of` indexes them by row.
        let expected_flags = match definition.format {
            CharDefFormat::Mecab => 0,
            CharDefFormat::Sudachi => definition.mapping.row_count(),
        };
        if definition.flags.len() != expected_flags {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "char_def.bin: {} flag entries for {} rows in the {:?} format",
                definition.flags.len(),
                definition.mapping.row_count(),
                definition.format
            )));
        }
        // The flat tables are not serialized; rebuild them after
        // deserialization.
        definition.build_flat_table();
        definition.build_flat_flags();
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

    /// Returns the categories of `c`.
    ///
    /// In the MeCab format, the first category is the default category of
    /// `c` (the first category of the last `char.def` line that covers it,
    /// MeCab's `default_type`), the only one that creates unknown-word
    /// candidates; the others follow in category id order (#1111). The list
    /// is empty only when no line covers `c` and `char.def` does not use
    /// `DEFAULT`.
    ///
    /// In the Sudachi format (#1388), every category of `c` comes in
    /// category id order: there is no default category, and each of them
    /// creates unknown-word candidates. The list is empty only when the
    /// lines that cover `c` give it flags and no category.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to look up.
    ///
    /// # Returns
    ///
    /// The category ids, the default category first in the MeCab format.
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
    /// `(offset, len)` pair instead of copying the categories (#942). The
    /// categories at those coordinates are those of
    /// [`Self::lookup_categories`], in the same order: the default category
    /// is at `offset`.
    ///
    /// # Arguments
    ///
    /// * `c` - The character to look up.
    ///
    /// # Returns
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
    /// # Arguments
    ///
    /// * `idx` - Pool index derived from [`Self::lookup_categories_packed`].
    ///
    /// # Returns
    ///
    /// The category id stored at that pool slot.
    #[inline(always)]
    pub(crate) fn flat_category(&self, idx: usize) -> CategoryId {
        self.flat_categories[idx]
    }
}

#[cfg(test)]
mod tests {
    /// `any_range_from` looks at every range that reaches `from` or beyond,
    /// including the open-ended last one, and at no range that ends below it.
    #[test]
    fn any_range_from_sees_only_ranges_reaching_from() {
        let mark = |cp: u32, out: &mut Vec<u8>| {
            if cp == 0x20 || cp == 0x3000 {
                out.push(1);
            }
        };
        let has_mark = |values: &[u8]| values.contains(&1);

        // 0x20..0x21 and 0x3000..0x3001 are marked.
        let table = LookupTable::from_fn(vec![0x20, 0x21, 0x3000, 0x3001], &mark);
        assert!(table.any_range_from(256, has_mark));
        assert!(table.any_range_from(0x3000, has_mark));
        assert!(!table.any_range_from(0x3001, has_mark));

        // Only 0x20..0x100 is marked: it ends below 256.
        let table = LookupTable::from_fn(vec![0x20, 0x100], &mark);
        assert!(table.any_range_from(0xFF, has_mark));
        assert!(!table.any_range_from(256, has_mark));

        // The last range is open-ended.
        let table = LookupTable::from_fn(vec![0x20], &mark);
        assert!(table.any_range_from(0x10_0000, has_mark));
    }

    use crate::dictionary::character_definition::{
        CategoryData, CategoryId, CharDefFormat, CharacterDefinition, LookupTable, NO_OOV_BOW,
        NO_OOV_EOW,
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

    /// A Sudachi-format definition over the rows of [`test_definition`],
    /// with flags on the row of 0x3040.. and on the row beyond the BMP.
    fn test_sudachi_definition() -> CharacterDefinition {
        let definition = test_definition();
        let flags = vec![0, 0, NO_OOV_BOW, 0, NO_OOV_BOW | NO_OOV_EOW];
        CharacterDefinition::new_sudachi(
            definition.category_definitions,
            definition.category_names,
            definition.mapping,
            flags,
        )
    }

    /// #1388: the flags of a code point are those of its row, through the
    /// flat BMP table and through the binary search beyond the BMP, and 0
    /// in the MeCab format.
    #[test]
    fn test_flags_of_every_code_point() {
        let sudachi = test_sudachi_definition();
        let mecab = test_definition();
        assert_eq!(sudachi.format(), CharDefFormat::Sudachi);
        assert_eq!(mecab.format(), CharDefFormat::Mecab);
        assert!(!sudachi.flat_flags.is_empty(), "flat flags should be built");
        for cp in 0..=0x10FFFFu32 {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            let expected = if cp >= 0x20000 {
                NO_OOV_BOW | NO_OOV_EOW
            } else if (0x3040..0x4E00).contains(&cp) {
                NO_OOV_BOW
            } else {
                0
            };
            assert_eq!(sudachi.flags_of(c), expected, "U+{cp:04X}");
            assert_eq!(mecab.flags_of(c), 0, "U+{cp:04X}");
        }
    }

    /// #1388: the format and the flags survive the round trip through
    /// `char_def.bin`, and the flat flag table is rebuilt by `load()`.
    #[test]
    fn test_sudachi_definition_survives_rkyv_round_trip() {
        let definition = test_sudachi_definition();
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&definition).unwrap();
        let reloaded = CharacterDefinition::load(&bytes).unwrap();
        assert_eq!(reloaded.format(), CharDefFormat::Sudachi);
        assert_eq!(reloaded.flags, definition.flags);
        assert!(!reloaded.flat_flags.is_empty(), "load() must rebuild");
        for cp in [0u32, 0x41, 0x3042, 0x4E8C, 0x20B9F] {
            let c = char::from_u32(cp).unwrap();
            assert_eq!(reloaded.flags_of(c), definition.flags_of(c));
            assert_eq!(
                reloaded.lookup_categories(c),
                definition.lookup_categories(c)
            );
        }
    }

    /// #1388: `load()` rejects flags that do not have one entry per row in
    /// the Sudachi format, or any flag in the MeCab format.
    #[test]
    fn test_load_rejects_flags_that_do_not_fit_the_rows() {
        let mut sudachi = test_sudachi_definition();
        sudachi.flags.pop();
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&sudachi).unwrap();
        assert!(CharacterDefinition::load(&bytes).is_err());

        let mut mecab = test_definition();
        mecab.flags = vec![0; mecab.mapping.row_count()];
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&mecab).unwrap();
        assert!(CharacterDefinition::load(&bytes).is_err());
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

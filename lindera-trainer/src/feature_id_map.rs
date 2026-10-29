//! The map from feature strings to the feature ids of a
//! [`FeatureExtractor`](crate::feature_extractor::FeatureExtractor).

use core::fmt;
use core::hash::BuildHasher;
use std::num::NonZeroU32;

use hashbrown::{DefaultHashBuilder, HashTable};

/// The feature strings that a
/// [`FeatureExtractor`](crate::feature_extractor::FeatureExtractor) has met,
/// with their ids.
///
/// The k-th string the map meets gets id k, from 1, and keeps it; a string
/// that training removed and that comes again later gets a new id, after
/// every id the map gave out before. The ids are therefore never reused.
///
/// A map holds millions of strings when it is trained on a full dictionary,
/// so the strings are kept back to back in one buffer, and the hash table
/// holds only the ids: the 2.04 M unigram features of mecab-ipadic took
/// 193 MiB as a `HashMap<String, NonZeroU32>`, with one allocation per
/// string (#1286). The table hashes with hashbrown's default hasher
/// (foldhash), seeded per map.
pub struct FeatureIdMap {
    /// The strings the map has met, back to back, in id order. The strings of
    /// removed features stay in it.
    buffer: String,
    /// The end in `buffer` of the string of each id, at `id - 1`; the string
    /// starts where the one of the previous id ends.
    ends: Vec<usize>,
    /// The ids of the live strings, each hashed by its string.
    table: HashTable<NonZeroU32>,
    /// The hasher of the strings.
    hasher: DefaultHashBuilder,
    /// The largest id that the map gives out: `u32::MAX`, except in tests.
    max_id: u32,
}

impl Default for FeatureIdMap {
    fn default() -> Self {
        Self::with_max_id(u32::MAX)
    }
}

impl FeatureIdMap {
    /// Creates an empty map that gives out ids up to `max_id`.
    ///
    /// # Arguments
    ///
    /// * `max_id` - The largest id to give out.
    ///
    /// # Returns
    ///
    /// The map.
    pub(crate) fn with_max_id(max_id: u32) -> Self {
        Self {
            buffer: String::new(),
            ends: Vec::new(),
            table: HashTable::new(),
            hasher: DefaultHashBuilder::default(),
            max_id,
        }
    }

    /// Returns the string of an id that the map gave out.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The strings of the map.
    /// * `ends` - The ends of the strings in `buffer`.
    /// * `id` - An id the map gave out, so that `ends` has an end for it.
    ///
    /// # Returns
    ///
    /// The string, live or removed.
    fn key<'a>(buffer: &'a str, ends: &[usize], id: NonZeroU32) -> &'a str {
        let index = id.get() as usize - 1;
        let start = index.checked_sub(1).map_or(0, |previous| ends[previous]);
        &buffer[start..ends[index]]
    }

    /// Returns the number of live features.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns whether the map has no live feature.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the id of a feature.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    ///
    /// # Returns
    ///
    /// The id, or `None` if the feature is not in the map.
    pub fn get(&self, feature: &str) -> Option<NonZeroU32> {
        let Self {
            buffer,
            ends,
            table,
            hasher,
            ..
        } = self;
        table
            .find(hasher.hash_one(feature), |&id| {
                Self::key(buffer, ends, id) == feature
            })
            .copied()
    }

    /// Returns whether a feature is in the map.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    ///
    /// # Returns
    ///
    /// `true` if the map has the feature.
    pub fn contains_key(&self, feature: &str) -> bool {
        self.get(feature).is_some()
    }

    /// Returns the feature of an id.
    ///
    /// # Arguments
    ///
    /// * `id` - The feature id.
    ///
    /// # Returns
    ///
    /// The feature string, or `None` if the map never gave out the id or its
    /// feature was removed, even when the same string came again later under
    /// a new id.
    pub fn resolve(&self, id: NonZeroU32) -> Option<&str> {
        if id.get() as usize > self.ends.len() {
            return None;
        }
        let feature = Self::key(&self.buffer, &self.ends, id);
        (self.get(feature) == Some(id)).then_some(feature)
    }

    /// Returns the live features with their ids, in ascending id order, which
    /// is the order the map met them in.
    ///
    /// # Returns
    ///
    /// The (feature, id) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, NonZeroU32)> + '_ {
        (1..=self.ends.len()).filter_map(|id| {
            let id = NonZeroU32::new(u32::try_from(id).ok()?)?;
            Some((self.resolve(id)?, id))
        })
    }

    /// Returns the id of a feature, giving it the next id if the map does not
    /// have it.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature string.
    ///
    /// # Returns
    ///
    /// The id, or `None` if the feature is new and the map has given out its
    /// largest id.
    pub(crate) fn intern(&mut self, feature: &str) -> Option<NonZeroU32> {
        let Self {
            buffer,
            ends,
            table,
            hasher,
            max_id,
        } = self;
        let hash = hasher.hash_one(feature);
        if let Some(&id) = table.find(hash, |&id| Self::key(buffer, ends, id) == feature) {
            return Some(id);
        }
        // The next id follows every id given out before, live or removed, so
        // that an id is never reused (see the type's documentation).
        let id = u32::try_from(ends.len() + 1)
            .ok()
            .filter(|&id| id <= *max_id)
            .and_then(NonZeroU32::new)?;
        buffer.push_str(feature);
        ends.push(buffer.len());
        table.insert_unique(hash, id, |&id| hasher.hash_one(Self::key(buffer, ends, id)));
        Some(id)
    }

    /// Removes the features whose ids `keep` rejects.
    ///
    /// # Arguments
    ///
    /// * `keep` - Returns whether to keep the feature of an id.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(NonZeroU32) -> bool) {
        self.table.retain(|id| keep(*id));
    }
}

impl fmt::Debug for FeatureIdMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Two maps are equal when they have the same live features with the same
/// ids, as two `HashMap`s are; the strings of removed features do not count.
impl PartialEq for FeatureIdMap {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .all(|(feature, id)| other.get(feature) == Some(id))
    }
}

impl Eq for FeatureIdMap {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN)
    }

    fn pairs(map: &FeatureIdMap) -> Vec<(String, u32)> {
        map.iter()
            .map(|(s, id)| (s.to_string(), id.get()))
            .collect()
    }

    #[test]
    fn ids_are_consecutive_in_first_seen_order() {
        let mut map = FeatureIdMap::default();
        for feature in ["b", "a", "b", "c", "a"] {
            map.intern(feature);
        }

        assert_eq!(
            pairs(&map),
            [
                ("b".to_string(), 1),
                ("a".to_string(), 2),
                ("c".to_string(), 3)
            ]
        );
        assert_eq!(map.get("a"), Some(id(2)));
        assert_eq!(map.get("d"), None);
        assert!(map.contains_key("c"));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn a_removed_feature_interned_again_gets_an_id_after_every_other() {
        let mut map = FeatureIdMap::default();
        for feature in ["a", "b", "c"] {
            map.intern(feature);
        }
        map.retain(|id| id.get() != 1);

        // Minting from the live count (2 + 1) would give `a` the id of `c`.
        assert_eq!(map.intern("a"), Some(id(4)));
        assert_eq!(map.intern("d"), Some(id(5)));
        assert_eq!(
            pairs(&map),
            [
                ("b".to_string(), 2),
                ("c".to_string(), 3),
                ("a".to_string(), 4),
                ("d".to_string(), 5),
            ]
        );
    }

    #[test]
    fn resolve_returns_only_live_ids() {
        let mut map = FeatureIdMap::default();
        for feature in ["a", "b"] {
            map.intern(feature);
        }
        map.retain(|id| id.get() != 1);
        map.intern("a");

        assert_eq!(
            map.resolve(id(1)),
            None,
            "removed, then interned again as 3"
        );
        assert_eq!(map.resolve(id(2)), Some("b"));
        assert_eq!(map.resolve(id(3)), Some("a"));
        assert_eq!(map.resolve(id(4)), None, "never given out");
    }

    #[test]
    fn iter_follows_the_ids_of_a_large_map() {
        // Large enough that walking the hash table would not give the ids in
        // order by chance.
        let mut map = FeatureIdMap::default();
        for n in 0..2000 {
            map.intern(&format!("f{n}"));
        }
        map.retain(|id| id.get() % 3 != 0);

        let ids: Vec<u32> = map.iter().map(|(_, id)| id.get()).collect();
        let expected: Vec<u32> = (1..=2000).filter(|id| id % 3 != 0).collect();
        assert_eq!(ids, expected);
        assert!(map.iter().all(|(s, id)| s == format!("f{}", id.get() - 1)));
    }

    #[test]
    fn strings_that_share_bytes_stay_apart() {
        let mut map = FeatureIdMap::default();
        for feature in ["", "a", "b", "ab", "ba"] {
            map.intern(feature);
        }

        assert_eq!(
            pairs(&map),
            [
                (String::new(), 1),
                ("a".to_string(), 2),
                ("b".to_string(), 3),
                ("ab".to_string(), 4),
                ("ba".to_string(), 5),
            ]
        );
        assert_eq!(map.resolve(id(1)), Some(""));
        assert_eq!(map.get("aba"), None);
    }

    #[test]
    fn equality_ignores_removed_features() {
        let mut trimmed = FeatureIdMap::default();
        for feature in ["a", "b"] {
            trimmed.intern(feature);
        }
        trimmed.retain(|id| id.get() != 1);
        let mut other = FeatureIdMap::default();
        for feature in ["x", "b"] {
            other.intern(feature);
        }
        other.retain(|id| id.get() != 1);

        assert_eq!(trimmed, other);
        other.intern("y");
        assert_ne!(trimmed, other);
        assert_eq!(format!("{trimmed:?}"), r#"{"b": 2}"#);
    }

    #[test]
    fn interning_past_the_largest_id_gives_none() {
        let mut map = FeatureIdMap::with_max_id(2);
        assert_eq!(map.intern("a"), Some(id(1)));
        assert_eq!(map.intern("b"), Some(id(2)));

        assert_eq!(map.intern("c"), None);
        // Known features keep their ids, and nothing was added.
        assert_eq!(map.intern("a"), Some(id(1)));
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("c"), None);
    }
}

use std::borrow::Cow;
use std::iter::{self, Chain, RepeatN};
use std::mem::ManuallyDrop;
use std::slice;

use lindera_dictionary::dictionary::{DetailFields, UNK};
use serde_json::{Value, json};

use crate::dictionary::{Dictionary, UserDictionary, WordId};

/// The field that pads a token's details to the dictionary schema's custom
/// field count, so that every token can be read by index up to that width.
const PADDING_FIELD: &str = "*";

/// What a morpheme is in its bunsetsu.
///
/// A bunsetsu has a head word, the last content word, and a function word,
/// the last particle or auxiliary; in a bunsetsu of content words alone
/// the two are the same morpheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BunsetsuRole {
    /// The head word of the bunsetsu.
    Head,
    /// The function word of the bunsetsu.
    Function,
    /// The head word and the function word at once.
    HeadAndFunction,
    /// Neither of the two.
    Other,
}

impl BunsetsuRole {
    /// The name of the role, as [`Token::as_value`] writes it.
    ///
    /// # Returns
    ///
    /// `head`, `function`, `head_function` or `other`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Function => "function",
            Self::HeadAndFunction => "head_function",
            Self::Other => "other",
        }
    }
}

/// The bunsetsu of a token and what that bunsetsu depends on: the result of
/// a dependency analysis, which a token filter writes.
///
/// A bunsetsu (文節) is a content word with the function words that follow
/// it. The bunsetsu of a sentence are numbered from 0, and each of them but
/// the last depends on a later one, its head.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenDependency {
    /// The number of the sentence in the text, from 0.
    pub sentence: u32,
    /// The number of the bunsetsu of the token in its sentence, from 0.
    pub bunsetsu: u32,
    /// The number of the bunsetsu that the bunsetsu of the token depends on,
    /// or `None` for the last bunsetsu of a sentence.
    pub head: Option<u32>,
    /// What the token is in its bunsetsu.
    pub role: BunsetsuRole,
    /// The score of the decision that attached the bunsetsu to its head; 0
    /// where there was nothing to decide.
    pub score: f64,
}

/// Where a token keeps its dependency: nothing, or a pointer to it.
///
/// A token of a pipeline without a dependency analysis has none, and what
/// such a token costs is what counts: the slot is one pointer, and its drop
/// is a test for a value that is not there. Freeing a value is in a function
/// of its own, out of line, so that the drop of a `Token` stays small enough
/// to be inlined where tokens are dropped. With an `Option<Box<_>>` as the
/// field, the drop of a `Token` became a call per token, and tokenizing took
/// 0.7% more instructions (#1438).
#[derive(Default)]
struct DependencySlot(ManuallyDrop<Option<Box<TokenDependency>>>);

impl Clone for DependencySlot {
    #[inline]
    fn clone(&self) -> Self {
        match &*self.0 {
            None => Self::default(),
            Some(dependency) => Self(ManuallyDrop::new(Some(dependency.clone()))),
        }
    }
}

impl Drop for DependencySlot {
    #[inline(always)]
    fn drop(&mut self) {
        if self.0.is_some() {
            free_dependency(&mut self.0);
        }
    }
}

/// Frees the dependency of a slot, which is left empty.
///
/// # Arguments
///
/// * `slot` - The value of a slot.
#[cold]
#[inline(never)]
fn free_dependency(slot: &mut Option<Box<TokenDependency>>) {
    *slot = None;
}

/// A token: a morpheme of the text, with its place in the text and its
/// entry in the dictionary.
///
/// A token is made by a tokenizer, or with [`Token::new`]. The type is
/// `#[non_exhaustive]`: outside this crate it cannot be built with a struct
/// expression, so that a field can be added without breaking that code.
/// Set the public fields after [`Token::new`] instead.
///
/// The result of a dependency analysis is read with [`Token::dependency`].
#[derive(Clone)]
#[non_exhaustive]
pub struct Token<'a> {
    /// The text content of the token, which is a copy-on-write string slice.
    /// This allows for efficient handling of both owned and borrowed string data.
    pub surface: Cow<'a, str>,

    /// The starting byte position of the token in the original text.
    /// This indicates where the token begins in the input string.
    pub byte_start: usize,

    /// The ending byte position of the token in the original text.
    /// This indicates the position immediately after the last byte of the token.
    pub byte_end: usize,

    /// This field represents the starting byte position of the token within the original input text.
    /// It is useful for mapping the token back to its location in the input.
    pub position: usize,

    /// The length of the token's position in the text.
    /// This indicates how many characters the token spans.
    pub position_length: usize,

    /// The identifier for the word, used to uniquely distinguish it within the context of the application.
    pub word_id: WordId,

    /// A reference to the dictionary used for tokenization.
    ///
    /// The dictionary contains the data necessary for the tokenization process,
    /// including word entries and their associated metadata. This reference
    /// allows the tokenizer to access and utilize the dictionary during
    /// the tokenization of input text.
    pub dictionary: &'a Dictionary,

    /// An optional reference to a user-defined dictionary.
    ///
    /// This dictionary can be used to add custom words or override existing words
    /// in the default dictionary. If `None`, the default dictionary is used.
    pub user_dictionary: Option<&'a UserDictionary>,

    /// The token's details, once they are loaded or written.
    ///
    /// Each element is a `Cow` (Copy-On-Write) string, so that the fields can
    /// borrow the dictionary's bytes or own a value a token filter wrote.
    ///
    /// # Note
    ///
    /// This field is `None` until the details are loaded or written:
    ///
    /// - [`Token::details`], [`Token::get_detail`] and [`Token::get`] load the
    ///   dictionary's details into it on their first call, so that repeated
    ///   reads by index do not split the dictionary entry again;
    /// - token filters that rewrite a token's details, such as the compound
    ///   word filters, store them here, and every accessor reads them from
    ///   here instead of the dictionary.
    ///
    /// [`Token::details_iter`] and [`Token::as_value`] read the details in one
    /// pass without loading them, so the field stays `None` after them; in
    /// particular after the Japanese part-of-speech filters, which read the
    /// details with [`Token::details_iter`].
    pub details: Option<Vec<Cow<'a, str>>>,

    /// The bunsetsu of the token and its dependency: see
    /// [`Token::dependency`] and [`Token::set_dependency`].
    dependency: DependencySlot,
}

impl<'a> Token<'a> {
    /// Creates a new `Token` instance with the provided parameters.
    ///
    /// # Arguments
    ///
    /// * `text` - A `Cow<'a, str>` representing the text of the token. This can be either a borrowed or owned string.
    /// * `start` - The byte position where the token starts in the original text.
    /// * `end` - The byte position where the token ends in the original text.
    /// * `position` - The position of the token in the sequence of tokens (usually an index).
    /// * `word_id` - The `WordId` associated with the token, identifying the token in the dictionary.
    /// * `dictionary` - A reference to the `Dictionary` that contains information about the token.
    /// * `user_dictionary` - An optional reference to a `UserDictionary`, which may provide additional user-defined tokens.
    ///
    /// # Returns
    ///
    /// Returns a new `Token` instance initialized with the provided values.
    ///
    /// # Details
    ///
    /// - The token's `text` can be a borrowed reference or an owned string, thanks to the use of `Cow<'a, str>`.
    /// - `byte_start` and `byte_end` are used to define the token's byte offset within the original text.
    /// - `position` marks the token's place in the overall tokenized sequence.
    /// - `position_length` is set to `1` by default.
    /// - `word_id` is used to identify the token in the dictionary, and the dictionaries (both `dictionary` and `user_dictionary`) provide additional details about the token.
    pub fn new(
        surface: Cow<'a, str>,
        start: usize,
        end: usize,
        position: usize,
        word_id: WordId,
        dictionary: &'a Dictionary,
        user_dictionary: Option<&'a UserDictionary>,
    ) -> Self {
        Self {
            surface,
            byte_start: start,
            byte_end: end,
            position,
            position_length: 1,
            word_id,
            dictionary,
            user_dictionary,
            details: None,
            dependency: DependencySlot::default(),
        }
    }

    /// The bunsetsu of the token and what that bunsetsu depends on.
    ///
    /// # Returns
    ///
    /// The result of the dependency analysis for this token, or `None`
    /// unless a token filter that analyzes dependencies ran.
    #[inline]
    pub fn dependency(&self) -> Option<&TokenDependency> {
        self.dependency.0.as_deref()
    }

    /// Sets or removes the dependency of the token. A token filter that
    /// analyzes dependencies calls this.
    ///
    /// # Arguments
    ///
    /// * `dependency` - The bunsetsu of the token and its dependency, or
    ///   `None` to remove what the token has.
    pub fn set_dependency(&mut self, dependency: Option<TokenDependency>) {
        *self.dependency.0 = dependency.map(Box::new);
    }

    /// Sets how many positions the token spans, which [`Token::new`] sets
    /// to 1.
    ///
    /// # Arguments
    ///
    /// * `position_length` - The number of positions, more than 1 for a
    ///   token that a filter merged from several.
    ///
    /// # Returns
    ///
    /// The token, with the length set.
    pub fn with_position_length(mut self, position_length: usize) -> Self {
        self.position_length = position_length;
        self
    }

    /// Sets the details of the token, in place of those of its dictionary
    /// entry.
    ///
    /// # Arguments
    ///
    /// * `details` - The detail fields, in the order of the schema of the
    ///   dictionary.
    ///
    /// # Returns
    ///
    /// The token, with the details set.
    pub fn with_details(mut self, details: Vec<Cow<'a, str>>) -> Self {
        self.details = Some(details);
        self
    }

    /// Retrieves the details of the token, either from the dictionary or the user-defined dictionary.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<&str>` containing the token's details. These details are typically part-of-speech information or other metadata about the token.
    ///
    /// # Process
    ///
    /// 1. **Check if details are already set**:
    ///    - If `self.details` is `None`, the method will attempt to fetch the details from either the system dictionary or the user dictionary.
    ///    - If the `word_id` is unknown, a default value `UNK` is returned.
    /// 2. **Fetch details from dictionaries**:
    ///    - If the `word_id` corresponds to a system dictionary entry, details are fetched from `self.dictionary`.
    ///    - If the `word_id` corresponds to a user-defined dictionary, details are fetched from `self.user_dictionary`.
    /// 3. **Store details**:
    ///    - The fetched details are stored in `self.details` as `Some(Vec<Cow<str>>)` to avoid recalculating them in subsequent calls.
    /// 4. **Return details as `&str`**:
    ///    - The `Cow<str>` values stored in `self.details` are converted to `&str` and returned.
    ///
    /// # Notes
    ///
    /// - The first time this method is called, it fetches the details from the dictionary (or user dictionary), but on subsequent calls, it returns the cached details in `self.details`.
    /// - If the token is unknown and no details can be retrieved, a default value (`UNK`) is used.
    /// - Loading the details allocates the cache once per token, and every call
    ///   collects a new `Vec<&str>`. To read the details once, in order, use
    ///   [`Token::details_iter`], which allocates neither.
    pub fn details(&mut self) -> Vec<&str> {
        // Ensure details are initialized
        self.ensure_details();

        // Fast path: return references without allocation
        match &self.details {
            Some(details) => details.iter().map(|x| x.as_ref()).collect(),
            None => UNK.to_vec(), // Fallback, should not happen after ensure_details()
        }
    }

    /// Returns an iterator over the token's details, read in one pass without
    /// allocating.
    ///
    /// The fields are those [`Token::details`] returns, in the same order and
    /// padded with `*` to the same width. They come from the `details` field
    /// when a token filter wrote it or a read by index loaded it, and
    /// otherwise straight from the dictionary's
    /// bytes, without loading them into the field: neither the cache nor the
    /// `Vec<&str>` that [`Token::details`] collects is allocated (#1399).
    /// Prefer this accessor when the details are consumed once in order
    /// (joining, serializing, copying into an FFI buffer).
    ///
    /// Since nothing is loaded, the `details` field stays `None` after this
    /// call when it was `None` before, and a [`Token::set_detail`] that
    /// follows it does nothing; call [`Token::details`] first to change a
    /// field.
    ///
    /// # Returns
    ///
    /// An iterator yielding each detail field as `&str`, in schema order.
    pub fn details_iter(&mut self) -> impl Iterator<Item = &str> {
        self.read_details()
    }

    /// One pass over the token's details: the stored ones when the
    /// `details` field is set, the dictionary's otherwise.
    ///
    /// Takes `&self`, so that [`Token::as_value`] can read the details while
    /// it reads the other fields; the public [`Token::details_iter`] keeps
    /// its `&mut self` signature.
    ///
    /// # Returns
    ///
    /// The fields in schema order, with an exact `size_hint`.
    fn read_details(&self) -> DetailsIter<'_, 'a> {
        match &self.details {
            Some(details) => DetailsIter::Stored(details.iter()),
            None => {
                let (fields, padding) = self.dictionary_fields();
                DetailsIter::Dictionary(fields.chain(iter::repeat_n(PADDING_FIELD, padding)))
            }
        }
    }

    /// The token's fields as its dictionary stores them, and how many `*`
    /// pad them to the dictionary schema's width.
    ///
    /// Details are padded to the dictionary schema's custom field count so
    /// that token filters can safely access any field by index.
    ///
    /// # Returns
    ///
    /// The fields of the token's system, unknown-word or user dictionary
    /// entry, borrowed from the dictionary's bytes for `'a`, and the number
    /// of `*` that follow them (0 when they already reach the schema's
    /// width).
    fn dictionary_fields(&self) -> (DetailFields<'a>, usize) {
        // Copied out of `self` so the fields borrow for `'a` rather than for
        // this `&self`; the dictionary accessors take `&'a self`.
        let dictionary: &'a Dictionary = self.dictionary;
        let user_dictionary: Option<&'a UserDictionary> = self.user_dictionary;
        let word_id = self.word_id.id() as usize;

        let fields = if self.word_id.is_unknown() {
            dictionary.unknown_word_details_iter(word_id)
        } else if self.word_id.is_system() {
            dictionary.word_details_iter(word_id)
        } else {
            match user_dictionary {
                Some(user_dictionary) => user_dictionary.word_details_iter(word_id),
                None => DetailFields::unk(),
            }
        };

        let expected_len = dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();
        let padding = expected_len.saturating_sub(fields.len());

        (fields, padding)
    }

    /// Loads and caches this token's detail fields, for the accessors that
    /// read them by index ([`Token::details`], [`Token::get_detail`],
    /// [`Token::get`]).
    ///
    /// The fields are borrowed straight out of the dictionary's own bytes and
    /// written into the cache vector, which is sized exactly once, so
    /// materializing a token's details costs a single allocation: the cache
    /// itself. Going through an intermediate `Vec<&str>` cost one more
    /// (#966).
    fn ensure_details(&mut self) {
        if self.details.is_some() {
            return;
        }

        // `DetailFields` is an `ExactSizeIterator`, so the padded width is
        // known before anything is materialized and this is the only
        // allocation on the path -- neither extend can reallocate.
        let (fields, padding) = self.dictionary_fields();
        let mut details: Vec<Cow<'a, str>> = Vec::with_capacity(fields.len() + padding);
        details.extend(fields.map(Cow::Borrowed));
        details.extend(iter::repeat_n(Cow::Borrowed(PADDING_FIELD), padding));

        self.details = Some(details);
    }

    /// Retrieves the token's detail at the specified index, if available.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the detail to retrieve.
    ///
    /// # Returns
    ///
    /// Returns an `Option<&str>` that contains the detail at the specified index.
    /// If the index is out of bounds or no details are available, `None` is returned.
    ///
    /// # Details
    ///
    /// - This method first loads the token's details into the `details` field, as [`Token::details`] does, unless they are there already; later reads by index use them without splitting the dictionary entry again.
    /// - If details are available and the provided index is valid, the detail at the specified index is returned as `Some(&str)`.
    /// - If the index is out of range, `None` is returned.
    pub fn get_detail(&mut self, index: usize) -> Option<&str> {
        self.ensure_details();
        self.details.as_ref()?.get(index).map(|c| c.as_ref())
    }

    /// Sets the token's detail at the specified index with the provided value.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the detail to set. This specifies which detail to update.
    /// * `detail` - A `Cow<'a, str>` representing the new detail value to set. It can either be a borrowed or owned string.
    ///
    /// # Details
    ///
    /// - If the token's details have already been populated (`self.details` is `Some`), this method updates the detail at the specified index.
    /// - If the provided index is valid (within bounds of the `details` vector), the detail at that index is replaced by the new `detail` value.
    /// - If the details have not been set (`self.details` is `None`), this method does nothing.
    /// - This method does not handle index out-of-bounds errors explicitly, so it assumes that the index provided is valid.
    ///
    /// # Notes
    ///
    /// - The `Cow<'a, str>` type allows flexibility, as it can handle either borrowed or owned strings.
    /// - This method does not initialize the details if they are not already set. To ensure the details are set, `details()` can be called prior to calling this method.
    /// - [`Token::details`], [`Token::get_detail`] and [`Token::get`] load the details; [`Token::details_iter`] and [`Token::as_value`] read them without loading them, so after those alone this method does nothing.
    pub fn set_detail(&mut self, index: usize, detail: Cow<'a, str>) {
        if let Some(details) = self.details.as_mut() {
            details[index] = detail;
        }
    }

    /// Retrieves the token's detail by field name.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The name of the field to retrieve.
    ///
    /// # Returns
    ///
    /// Returns an `Option<&str>` containing the value of the specified field.
    /// If the field name is not found or the schema is not available, `None` is returned.
    ///
    /// A field of the dictionary's schema is read with [`Token::get_detail`],
    /// which loads the token's details into the `details` field.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lindera_segmenter::token::Token;
    /// # let mut token: Token = unimplemented!();
    /// let base_form = token.get("base_form");
    /// let pos = token.get("major_pos");
    /// ```
    pub fn get(&mut self, field_name: &str) -> Option<&str> {
        // Get field index from schema
        let index = self
            .dictionary
            .metadata
            .dictionary_schema
            .get_field_index(field_name)?;

        // Handle common fields
        match index {
            0 => Some(self.surface.as_ref()), // surface
            1..=3 => None, // left_context_id, right_context_id, cost are not stored in token
            _ => {
                // For custom fields (index >= 4), get from details
                // details array doesn't include the first 4 common fields
                self.get_detail(index - 4)
            }
        }
    }

    /// Returns all token fields as a JSON Value.
    ///
    /// # Returns
    ///
    /// Returns a `serde_json::Value` object with the token's `surface`,
    /// `byte_start`, `byte_end` and `word_id`, and one entry per field of the
    /// dictionary's schema that the token's details reach. `byte_start`,
    /// `byte_end` and `word_id` are numbers. The dictionary's fields are
    /// strings, with the text the dictionary has, even when it looks like a
    /// number: `007` stays `"007"`, and an identifier beyond 2^53, such as
    /// UniDic CWJ's `lexicon_id`, comes back exactly from readers that hold
    /// numbers as doubles (#1365).
    ///
    /// The details are read in one pass, as [`Token::details_iter`] reads
    /// them, without loading them into the `details` field (#1399).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lindera_segmenter::token::Token;
    /// # let mut token: Token = unimplemented!();
    /// let value = token.as_value();
    /// println!("Surface: {}", value["surface"]);
    /// println!("Byte start: {}", value["byte_start"]); // This is a number
    /// println!("Word ID: {}", value["word_id"]); // This is a number
    /// ```
    pub fn as_value(&mut self) -> Value {
        let schema_custom_fields = self
            .dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields();

        // Build JSON object
        let mut obj = serde_json::Map::new();

        // Add surface as string
        obj.insert("surface".to_string(), json!(&*self.surface));

        // Add byte positions as numbers
        obj.insert("byte_start".to_string(), json!(self.byte_start));
        obj.insert("byte_end".to_string(), json!(self.byte_end));

        // Add word_id as number
        obj.insert("word_id".to_string(), json!(self.word_id.id()));

        // Add each field of the schema that the details reach, as the text
        // the dictionary has: a field that looks like a number in one entry
        // may not in another, and the text may not survive a number (`007`,
        // `NaN`, identifiers beyond 2^53), so it is never parsed (#1365).
        for (field_name, value) in schema_custom_fields.iter().zip(self.read_details()) {
            obj.insert(field_name.to_string(), json!(value));
        }

        // The dependency, when a token filter wrote one: numbers as numbers,
        // and `null` for the head of the last bunsetsu of a sentence.
        if let Some(dependency) = self.dependency() {
            obj.insert(
                "dependency".to_string(),
                json!({
                    "sentence": dependency.sentence,
                    "bunsetsu": dependency.bunsetsu,
                    "head": dependency.head,
                    "role": dependency.role.as_str(),
                    "score": dependency.score,
                }),
            );
        }

        Value::Object(obj)
    }
}

/// One pass over a token's details, as [`Token::details_iter`] yields them.
///
/// Either the details stored in the token's `details` field, or its
/// dictionary's fields followed by the `*` that pad them to the schema's
/// width, read without loading them into the field.
enum DetailsIter<'t, 'a> {
    /// The details in the token's `details` field.
    Stored(slice::Iter<'t, Cow<'a, str>>),
    /// The dictionary's fields, then their padding.
    Dictionary(Chain<DetailFields<'a>, RepeatN<&'a str>>),
}

impl<'t, 'a: 't> Iterator for DetailsIter<'t, 'a> {
    type Item = &'t str;

    /// Yields the next detail field.
    ///
    /// # Returns
    ///
    /// The next field in schema order, or `None` once all are read.
    #[inline]
    fn next(&mut self) -> Option<&'t str> {
        match self {
            Self::Stored(details) => details.next().map(|detail| detail.as_ref()),
            Self::Dictionary(fields) => fields.next(),
        }
    }

    /// Reports the exact number of fields left, so that collecting the
    /// details allocates once.
    ///
    /// # Returns
    ///
    /// `(remaining, Some(remaining))`.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Stored(details) => details.size_hint(),
            Self::Dictionary(fields) => fields.size_hint(),
        }
    }
}

#[cfg(all(test, feature = "embed-ipadic"))]
mod tests {
    use std::borrow::Cow;

    use lindera_dictionary::mode::Mode;

    use lindera_dictionary::viterbi::LexType;

    use super::{BunsetsuRole, DependencySlot, Token, TokenDependency};
    use crate::dictionary::{WordId, load_dictionary};
    use crate::segmenter::Segmenter;

    /// `Dictionary::word_details` must return every field the entry carries,
    /// in schema order, for a real dictionary. IPADIC's schema is 13 fields
    /// (4 common + 9 custom), so a system entry yields exactly 9 details --
    /// the shape the presized vector in `word_details` depends on (#966).
    #[test]
    fn test_word_details_field_count_and_order() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let expected_len = dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();
        assert_eq!(
            expected_len, 9,
            "IPADIC is expected to carry 9 custom fields"
        );

        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokens = match segmenter.segment(Cow::Borrowed("東京都に住む")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert!(!tokens.is_empty());

        for token in &tokens {
            if !token.word_id.is_system() || token.word_id.id() == u32::MAX {
                continue;
            }
            let details = token.dictionary.word_details(token.word_id.id() as usize);
            assert_eq!(
                details.len(),
                expected_len,
                "surface {:?} yielded {details:?}",
                token.surface
            );
            // The first field is the major POS, never empty for a system entry.
            assert!(!details[0].is_empty(), "surface {:?}", token.surface);
        }
    }

    /// An out-of-range system word id yields an empty vector, while the user
    /// dictionary's accessor falls back to the `UNK` sentinel. The two
    /// fallbacks differ and both are load-bearing, so pin them here.
    #[test]
    fn test_word_details_out_of_range_returns_empty() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        assert!(dictionary.word_details(usize::MAX / 4).is_empty());
    }

    /// A user-dictionary entry declares only `surface, part_of_speech,
    /// reading` (IPADIC's `user_dictionary_schema`), but the builder already
    /// expands it onto the system schema's field positions, padding the rest
    /// with `*`. So its details come back at the full custom-field width,
    /// with the POS at index 0 and the reading at index 7 -- the positions
    /// `part_of_speech` and `reading` occupy in the system schema. Pin that
    /// here: `set_detail` indexes this vector directly, so any narrowing
    /// would panic downstream (#966).
    #[test]
    fn test_user_dictionary_details_match_system_schema_positions() {
        use std::path::PathBuf;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = match Segmenter::from_config(&config) {
            Ok(segmenter) => segmenter,
            Err(err) => panic!("failed to build segmenter: {err}"),
        };
        let expected_len = segmenter
            .dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();

        let tokens = match segmenter.segment(Cow::Borrowed("東京スカイツリーの近く")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };

        let mut saw_user_token = false;
        for mut token in tokens {
            if token.word_id.is_system() || token.word_id.is_unknown() {
                continue;
            }
            saw_user_token = true;
            let surface = token.surface.to_string();
            let details = token.details();
            assert_eq!(
                details.len(),
                expected_len,
                "user token {surface:?} yielded {details:?}"
            );
            // `part_of_speech` is custom field 0, `reading` is custom field 7.
            assert_eq!(details[0], "カスタム名詞", "{details:?}");
            assert_eq!(details[7], "トウキョウスカイツリー", "{details:?}");
            // Every position the user schema does not declare is filled with "*".
            assert!(
                details[1..7]
                    .iter()
                    .chain(details[8..].iter())
                    .all(|d| *d == "*"),
                "{details:?}"
            );
        }
        assert!(
            saw_user_token,
            "expected at least one user-dictionary token"
        );
    }

    /// `details_iter` must yield exactly the same fields, in the same
    /// order, as the `Vec`-collecting `details()` accessor, for both known
    /// and unknown words.
    #[test]
    fn test_details_iter_matches_details() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

        // Mixed input: dictionary words plus an unknown (Latin) token.
        let tokens = match segmenter.segment(Cow::Borrowed("すもももsupercalifragilisticも")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert!(!tokens.is_empty());

        for mut token in tokens {
            let expected: Vec<String> = {
                let mut probe = token.clone();
                probe.details().iter().map(|s| s.to_string()).collect()
            };
            let actual: Vec<String> = token.details_iter().map(|s| s.to_string()).collect();
            assert_eq!(expected, actual, "surface {:?}", token.surface);
            assert!(!actual.is_empty());
        }
    }

    /// The detail cache must be allocated exactly once: `DetailFields` is an
    /// `ExactSizeIterator`, so `ensure_details` can size the vector before
    /// materializing anything and neither the extend nor the schema padding
    /// reallocates. A capacity larger than the padded width means something
    /// grew, which is precisely the regression #966 step (b) removed -- and
    /// it is observable without an allocator hook.
    #[test]
    fn test_details_cache_is_allocated_exactly_once() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let expected_len = dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();

        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        // Mixed input: system entries plus a Latin run that becomes an
        // unknown word, so both dictionary paths are covered.
        let tokens = match segmenter.segment(Cow::Borrowed("東京都でsupercalifragilisticを買う"))
        {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert!(!tokens.is_empty());

        let mut saw_unknown = false;
        for mut token in tokens {
            saw_unknown |= token.word_id.is_unknown();
            let _ = token.details();
            let details = match token.details.as_ref() {
                Some(details) => details,
                None => panic!("details must be cached after details()"),
            };
            assert_eq!(details.len(), expected_len, "surface {:?}", token.surface);
            assert_eq!(
                details.capacity(),
                expected_len,
                "the cache reallocated for surface {:?}",
                token.surface
            );
        }
        assert!(saw_unknown, "expected at least one unknown-word token");
    }

    /// The same, on the user-dictionary path, which reaches a third accessor.
    #[test]
    fn test_user_dictionary_details_cache_is_allocated_exactly_once() {
        use std::path::PathBuf;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");

        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });

        let segmenter = match Segmenter::from_config(&config) {
            Ok(segmenter) => segmenter,
            Err(err) => panic!("failed to build segmenter: {err}"),
        };
        let expected_len = segmenter
            .dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();

        let tokens = match segmenter.segment(Cow::Borrowed("東京スカイツリーの近く")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };

        let mut saw_user_token = false;
        for mut token in tokens {
            if !token.word_id.is_system() && !token.word_id.is_unknown() {
                saw_user_token = true;
            }
            let _ = token.details();
            let details = match token.details.as_ref() {
                Some(details) => details,
                None => panic!("details must be cached after details()"),
            };
            assert_eq!(details.capacity(), expected_len);
        }
        assert!(
            saw_user_token,
            "expected at least one user-dictionary token"
        );
    }

    /// Text whose tokens come from all three dictionaries of
    /// [`mixed_segmenter`]: a user dictionary word (`東京スカイツリー`), system
    /// words, and a Latin run that becomes an unknown word.
    const MIXED_TEXT: &str = "東京スカイツリーの近くでsupercalifragilistic";

    /// A segmenter for IPADIC with `resources/user_dict/ipadic_simple_userdic.csv`.
    fn mixed_segmenter() -> Segmenter {
        use std::path::PathBuf;

        let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");
        let config = serde_json::json!({
            "dictionary": "embedded://ipadic",
            "user_dictionary": userdic_file.to_str().unwrap(),
            "mode": "normal"
        });
        match Segmenter::from_config(&config) {
            Ok(segmenter) => segmenter,
            Err(err) => panic!("failed to build segmenter: {err}"),
        }
    }

    /// `details_iter` reads a token's details without loading them into the
    /// `details` field (#1399), and yields the fields `details()` loads,
    /// padded to the schema's width, for a system, an unknown and a user
    /// dictionary word.
    #[test]
    fn test_details_iter_leaves_details_unloaded() {
        let segmenter = mixed_segmenter();
        let width = segmenter
            .dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();
        let tokens = match segmenter.segment(Cow::Borrowed(MIXED_TEXT)) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };

        let (mut saw_system, mut saw_unknown, mut saw_user) = (false, false, false);
        for mut token in tokens {
            if token.word_id.is_unknown() {
                saw_unknown = true;
            } else if token.word_id.is_system() {
                saw_system = true;
            } else {
                saw_user = true;
            }

            let loaded: Vec<String> = {
                let mut probe = token.clone();
                probe.details().iter().map(|s| s.to_string()).collect()
            };
            let read: Vec<String> = token.details_iter().map(str::to_string).collect();
            assert_eq!(read, loaded, "surface {:?}", token.surface);
            assert_eq!(read.len(), width, "surface {:?}", token.surface);
            assert!(
                token.details.is_none(),
                "details_iter loaded the details of {:?}",
                token.surface
            );
        }
        assert!(saw_system, "expected a system dictionary token");
        assert!(saw_unknown, "expected an unknown-word token");
        assert!(saw_user, "expected a user dictionary token");
    }

    /// `as_value` reads the details without loading them into the `details`
    /// field (#1399), and gives the value it gives for a token whose details
    /// were loaded first.
    #[test]
    fn test_as_value_leaves_details_unloaded() {
        let segmenter = mixed_segmenter();
        let tokens = match segmenter.segment(Cow::Borrowed(MIXED_TEXT)) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert!(!tokens.is_empty());

        for mut token in tokens {
            let expected = {
                let mut loaded = token.clone();
                let _ = loaded.details();
                assert!(loaded.details.is_some());
                loaded.as_value()
            };
            let value = token.as_value();
            assert_eq!(value, expected, "surface {:?}", token.surface);
            assert!(
                token.details.is_none(),
                "as_value loaded the details of {:?}",
                token.surface
            );
        }
    }

    /// Details written into the `details` field, as the compound word filters
    /// write them, are what `details_iter` and `as_value` read, not the
    /// dictionary's.
    #[test]
    fn test_single_passes_read_the_stored_details() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let mut tokens = match segmenter.segment(Cow::Borrowed("東京")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert_eq!(tokens.len(), 1);
        let mut token = tokens.remove(0);
        token.details = Some(vec![
            Cow::Borrowed("複合語"),
            Cow::Owned("書き込み".to_string()),
        ]);

        let read: Vec<&str> = token.details_iter().collect();
        assert_eq!(read, ["複合語", "書き込み"]);

        let value = token.as_value();
        assert_eq!(value["part_of_speech"], "複合語");
        assert_eq!(value["part_of_speech_subcategory_1"], "書き込み");
        assert!(value.get("part_of_speech_subcategory_2").is_none());
    }

    /// `details_iter` reports its exact length before anything is read, so
    /// that collecting the details, as the bindings' `TokenView` does,
    /// allocates once: for the dictionary's fields with their padding and for
    /// stored details alike.
    #[test]
    fn test_details_iter_reports_its_length() {
        let segmenter = mixed_segmenter();
        let mut tokens = match segmenter.segment(Cow::Borrowed(MIXED_TEXT)) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };
        assert!(!tokens.is_empty());
        // Stored details narrower than the schema, as a merged token has.
        tokens[0].details = Some(vec![Cow::Borrowed("複合語")]);

        for mut token in tokens {
            let len = token.clone().details().len();
            let hint = token.details_iter().size_hint();
            assert_eq!(hint, (len, Some(len)), "surface {:?}", token.surface);
        }
    }

    /// IPADIC stores 活用型 (conjugation type) before 活用形 (conjugation
    /// form), so the name-based accessors must resolve `conjugation_type` to
    /// the first of the two columns and `conjugation_form` to the second.
    /// The schema had the names the other way round until #1086.
    #[test]
    fn test_ipadic_conjugation_fields_by_name() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokens = match segmenter.segment(Cow::Borrowed("書いた")) {
            Ok(tokens) => tokens,
            Err(err) => panic!("segment failed: {err}"),
        };

        let expected = [
            ("書い", "五段・カ行イ音便", "連用タ接続"),
            ("た", "特殊・タ", "基本形"),
        ];
        assert_eq!(tokens.len(), expected.len());

        for (mut token, (surface, conjugation_type, conjugation_form)) in
            tokens.into_iter().zip(expected)
        {
            assert_eq!(token.surface, surface);
            assert_eq!(token.get("conjugation_type"), Some(conjugation_type));
            assert_eq!(token.get("conjugation_form"), Some(conjugation_form));

            let value = token.as_value();
            assert_eq!(value["conjugation_type"], conjugation_type);
            assert_eq!(value["conjugation_form"], conjugation_form);
        }
    }

    /// The fields of [`NUMERIC_LOOKING_ENTRY`] and the text of each, which
    /// `as_value` must give back as strings.
    const NUMERIC_LOOKING_FIELDS: [(&str, &str); 9] = [
        ("part_of_speech", "名詞"),
        ("part_of_speech_subcategory_1", "007"),
        ("part_of_speech_subcategory_2", "NaN"),
        ("part_of_speech_subcategory_3", "1e3"),
        ("conjugation_type", "-1"),
        ("conjugation_form", "0.5"),
        ("base_form", "9726013605028523"),
        ("reading", "inf"),
        ("pronunciation", "スウジゴ"),
    ];

    /// A detailed user dictionary entry in IPADIC's columns whose details
    /// look like numbers, as in [`NUMERIC_LOOKING_FIELDS`].
    const NUMERIC_LOOKING_ENTRY: &str =
        "数字語,1288,1288,-1000,名詞,007,NaN,1e3,-1,0.5,9726013605028523,inf,スウジゴ\n";

    /// `as_value` writes the dictionary's fields as the text the dictionary
    /// has (#1365). Until then it wrote `007` as `7`, `NaN` and `inf` as
    /// `null`, `1e3` as `1000.0`, and `9726013605028523` as a number, which
    /// readers with doubles read as `9726013605028524`.
    #[test]
    fn test_as_value_writes_the_dictionary_fields_as_strings() {
        let dictionary = match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => dictionary,
            Err(err) => panic!("failed to load embedded IPADIC: {err}"),
        };
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("numeric_looking_userdic.csv");
        std::fs::write(&csv, NUMERIC_LOOKING_ENTRY).unwrap();
        let user_dictionary =
            crate::dictionary::load_user_dictionary_from_csv(&dictionary.metadata, &csv).unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
        let mut tokens = segmenter.segment(Cow::Borrowed("数字語")).unwrap();
        assert_eq!(tokens.len(), 1);

        let value = tokens[0].as_value();
        assert_eq!(value["surface"], "数字語");
        for (field, text) in NUMERIC_LOOKING_FIELDS {
            assert_eq!(value[field], serde_json::json!(text), "{field}");
        }
    }

    /// A token a tokenizer makes has no dependency, and its JSON has no
    /// `dependency` key: the output of a pipeline without the filter is what
    /// it was before the field (#1438).
    #[test]
    fn test_a_token_has_no_dependency_by_default() {
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let mut tokens = segmenter.segment(Cow::Borrowed("彼は本を読んだ")).unwrap();
        for token in tokens.iter_mut() {
            assert!(token.dependency().is_none());
            let value = token.as_value();
            let keys: Vec<&str> = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert!(!keys.contains(&"dependency"), "{keys:?}");
            // The four fields of every token, and the nine of IPADIC.
            assert_eq!(keys.len(), 13, "{keys:?}");
        }
    }

    /// With a dependency, the JSON has it as an object of numbers and a
    /// name, next to the fields of the dictionary, and the details are those
    /// of the dictionary still.
    #[test]
    fn test_as_value_writes_the_dependency() {
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let mut tokens = segmenter.segment(Cow::Borrowed("彼は読んだ")).unwrap();
        let without: Vec<serde_json::Value> = tokens.iter_mut().map(Token::as_value).collect();
        let details: Vec<String> = tokens[0].details_iter().map(str::to_string).collect();

        tokens[0].set_dependency(Some(TokenDependency {
            sentence: 2,
            bunsetsu: 0,
            head: Some(1),
            role: BunsetsuRole::Head,
            score: 0.75,
        }));
        let last = tokens.len() - 1;
        tokens[last].set_dependency(Some(TokenDependency {
            sentence: 2,
            bunsetsu: 1,
            head: None,
            role: BunsetsuRole::Other,
            score: 0.0,
        }));

        let value = tokens[0].as_value();
        assert_eq!(
            value["dependency"],
            serde_json::json!({
                "sentence": 2, "bunsetsu": 0, "head": 1, "role": "head", "score": 0.75
            })
        );
        // Everything else is as without the dependency.
        let mut rest = value.clone();
        rest.as_object_mut().unwrap().remove("dependency");
        assert_eq!(rest, without[0]);
        assert_eq!(
            tokens[0].details_iter().collect::<Vec<_>>(),
            details.iter().map(String::as_str).collect::<Vec<_>>()
        );

        // The last bunsetsu of a sentence has no head: `null`, not -1.
        let value = tokens[last].as_value();
        assert_eq!(value["dependency"]["head"], serde_json::Value::Null);
        assert_eq!(value["dependency"]["role"], "other");
        // A token in between has none.
        assert!(tokens[1].as_value().get("dependency").is_none());
        // A clone of a token has the dependency of the token, and one of a
        // token without it has none.
        assert_eq!(tokens[0].clone().dependency(), tokens[0].dependency());
        assert!(tokens[1].clone().dependency().is_none());
        // Setting it again replaces it, and `None` removes it.
        let mut token = tokens[0].clone();
        token.set_dependency(tokens[last].dependency().cloned());
        assert_eq!(token.dependency().unwrap().bunsetsu, 1);
        token.set_dependency(None);
        assert!(token.dependency().is_none());
        assert!(token.as_value().get("dependency").is_none());
    }

    /// The names of the roles are part of the JSON output.
    #[test]
    fn test_the_roles_have_names() {
        assert_eq!(BunsetsuRole::Head.as_str(), "head");
        assert_eq!(BunsetsuRole::Function.as_str(), "function");
        assert_eq!(BunsetsuRole::HeadAndFunction.as_str(), "head_function");
        assert_eq!(BunsetsuRole::Other.as_str(), "other");
    }

    /// `Token::new` and the two `with_` methods make what a struct
    /// expression made before the type was `#[non_exhaustive]`.
    #[test]
    fn test_a_token_is_built_without_a_struct_expression() {
        let dictionary = load_dictionary("embedded://ipadic").unwrap();
        let word_id = WordId::new(LexType::System, 0);
        let mut token = Token::new(Cow::Borrowed("本"), 3, 6, 4, word_id, &dictionary, None);
        assert_eq!(token.position_length, 1);
        assert!(token.details.is_none());
        assert!(token.dependency().is_none());

        token = token
            .with_position_length(2)
            .with_details(vec![Cow::Borrowed("名詞"), Cow::Borrowed("一般")]);
        assert_eq!(token.surface, "本");
        assert_eq!(
            (token.byte_start, token.byte_end, token.position),
            (3, 6, 4)
        );
        assert_eq!(token.position_length, 2);
        assert_eq!(token.details(), ["名詞", "一般"]);
    }

    /// The dependency is behind a pointer: a token grows by 8 bytes, whatever
    /// the result holds.
    #[test]
    fn test_the_dependency_costs_a_token_one_pointer() {
        assert_eq!(
            std::mem::size_of::<DependencySlot>(),
            std::mem::size_of::<usize>()
        );
    }
}

use std::borrow::Cow;

use lindera_dictionary::dictionary::{DetailFields, UNK};
use serde_json::{Value, json};

use crate::dictionary::{Dictionary, UserDictionary, WordId};

#[derive(Clone)]
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

    /// An optional vector containing detailed information about the token.
    /// Each element in the vector is a `Cow` (Copy-On-Write) type, which allows
    /// for efficient handling of both owned and borrowed string data.
    ///
    /// # Note
    ///
    /// This field is optional and may be `None` if no detailed information is available.
    pub details: Option<Vec<Cow<'a, str>>>,
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
        }
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
    pub fn details(&mut self) -> Vec<&str> {
        // Ensure details are initialized
        self.ensure_details();

        // Fast path: return references without allocation
        match &self.details {
            Some(details) => details.iter().map(|x| x.as_ref()).collect(),
            None => UNK.to_vec(), // Fallback, should not happen after ensure_details()
        }
    }

    /// Returns an iterator over the token's details without allocating the
    /// intermediate `Vec<&str>` that [`Token::details`] collects on every
    /// call.
    ///
    /// Details are loaded (and cached) on first access, exactly as with
    /// [`Token::details`]; only the per-call collection is avoided. Prefer
    /// this accessor when the details are consumed once in order (joining,
    /// serializing, copying into an FFI buffer).
    ///
    /// # 戻り値
    ///
    /// An iterator yielding each detail field as `&str`, in schema order.
    pub fn details_iter(&mut self) -> impl Iterator<Item = &str> {
        self.ensure_details();
        self.details
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|c| c.as_ref())
    }

    /// Loads and caches this token's detail fields.
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

        // Copied out of `self` before the borrow below so the fields borrow
        // for `'a` rather than for this `&mut self`; the dictionary accessors
        // take `&'a self`.
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

        // Details are padded to the dictionary schema's custom field count so
        // that token filters can safely access any field by index.
        let expected_len = dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields()
            .len();

        // `DetailFields` is an `ExactSizeIterator`, so the padded width is
        // known before anything is materialized and this is the only
        // allocation on the path -- neither the extend nor the pad below can
        // reallocate.
        let mut details: Vec<Cow<'a, str>> = Vec::with_capacity(fields.len().max(expected_len));
        details.extend(fields.map(Cow::Borrowed));
        if details.len() < expected_len {
            details.resize(expected_len, Cow::Borrowed("*"));
        }

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
    /// - This method first ensures that the token's details are populated by calling `self.details()`.
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
    /// # Example
    ///
    /// ```no_run
    /// # use lindera::token::Token;
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
    /// Returns a `serde_json::Value` containing all available fields and their values.
    /// Numeric fields (byte_start, byte_end, word_id) are represented as numbers,
    /// while text fields remain as strings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use lindera::token::Token;
    /// # let mut token: Token = unimplemented!();
    /// let value = token.as_value();
    /// println!("Surface: {}", value["surface"]);
    /// println!("Byte start: {}", value["byte_start"]); // This is a number
    /// println!("Word ID: {}", value["word_id"]); // This is a number
    /// ```
    pub fn as_value(&mut self) -> Value {
        // Get schema info first
        let schema_custom_fields = self
            .dictionary
            .metadata
            .dictionary_schema
            .get_custom_fields();

        // Copy values before mutable borrow
        let surface = self.surface.to_string();
        let byte_start = self.byte_start;
        let byte_end = self.byte_end;
        let word_id = self.word_id.id();

        // Get details (requires mutable borrow)
        let details = self.details();

        // Build JSON object
        let mut obj = serde_json::Map::new();

        // Add surface as string
        obj.insert("surface".to_string(), json!(surface));

        // Add byte positions as numbers
        obj.insert("byte_start".to_string(), json!(byte_start));
        obj.insert("byte_end".to_string(), json!(byte_end));

        // Add word_id as number
        obj.insert("word_id".to_string(), json!(word_id));

        // Add each custom field from the schema
        for (i, field_name) in schema_custom_fields.iter().enumerate() {
            if let Some(value) = details.get(i) {
                // Try to parse as number if possible, otherwise keep as string
                if let Ok(num) = value.parse::<i64>() {
                    obj.insert(field_name.to_string(), json!(num));
                } else if let Ok(num) = value.parse::<f64>() {
                    obj.insert(field_name.to_string(), json!(num));
                } else {
                    obj.insert(field_name.to_string(), json!(*value));
                }
            }
        }

        Value::Object(obj)
    }
}

#[cfg(all(test, feature = "embed-ipadic"))]
mod tests {
    use std::borrow::Cow;

    use lindera_dictionary::mode::Mode;

    use crate::dictionary::load_dictionary;
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
            let _ = token.details_iter().count();
            let details = match token.details.as_ref() {
                Some(details) => details,
                None => panic!("details must be cached after details_iter"),
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
            let _ = token.details_iter().count();
            let details = match token.details.as_ref() {
                Some(details) => details,
                None => panic!("details must be cached after details_iter"),
            };
            assert_eq!(details.capacity(), expected_len);
        }
        assert!(
            saw_user_token,
            "expected at least one user-dictionary token"
        );
    }
}

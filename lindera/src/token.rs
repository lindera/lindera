use std::borrow::Cow;

use lindera_dictionary::dictionary::UNK;
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

    /// Helper method to ensure details are loaded without returning them
    fn ensure_details(&mut self) {
        if self.details.is_none() {
            let tmp = if self.word_id.is_unknown() {
                self.dictionary
                    .unknown_word_details(self.word_id.id as usize)
            } else if self.word_id.is_system() {
                self.dictionary.word_details(self.word_id.id as usize)
            } else {
                match self.user_dictionary {
                    Some(user_dictionary) => user_dictionary.word_details(self.word_id.id as usize),
                    None => UNK.to_vec(),
                }
            };

            let mut details: Vec<Cow<'a, str>> = tmp.into_iter().map(Cow::Borrowed).collect();

            // Pad details to match the dictionary schema's custom field count so that
            // token filters can safely access any field by index.
            let expected_len = self
                .dictionary
                .metadata
                .dictionary_schema
                .get_custom_fields()
                .len();
            if details.len() < expected_len {
                details.resize(expected_len, Cow::Borrowed("*"));
            }

            self.details = Some(details);
        }
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
        self.details().get(index).copied()
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
        let word_id = self.word_id.id;

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

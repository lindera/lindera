use std::borrow::Cow;

use serde::Serialize;

use crate::dictionary::WordId;
use lindera_dictionary::dictionary::{Dictionary, UserDictionary, UNK};

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    /// The text content of the token, which is a copy-on-write string slice.
    /// This allows for efficient handling of both owned and borrowed string data.
    pub text: Cow<'a, str>,

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
        text: Cow<'a, str>,
        start: usize,
        end: usize,
        position: usize,
        word_id: WordId,
        dictionary: &'a Dictionary,
        user_dictionary: Option<&'a UserDictionary>,
    ) -> Self {
        Self {
            text,
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
        // set details if it is not set yet.
        if self.details.is_none() {
            let tmp = if self.word_id.is_unknown() {
                UNK.to_vec()
            } else if self.word_id.is_system() {
                self.dictionary.word_details(self.word_id.0 as usize)
            } else {
                match self.user_dictionary {
                    Some(user_dictionary) => user_dictionary.word_details(self.word_id.0 as usize),
                    None => UNK.to_vec(),
                }
            };

            self.details = Some(tmp.into_iter().map(Cow::Borrowed).collect());
        }

        // convert Cow to &str.
        self.details
            .as_ref()
            .map(|vec| vec.iter().map(|x| x.as_ref()).collect())
            .unwrap_or_else(|| UNK.to_vec())
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
}

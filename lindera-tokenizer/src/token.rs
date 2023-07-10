use byteorder::{ByteOrder, LittleEndian};
use once_cell::sync::Lazy;
use serde::Serialize;

use lindera_core::{
    dictionary::{Dictionary, UserDictionary},
    word_entry::WordId,
};

static UNK: Lazy<Vec<&str>> = Lazy::new(|| vec!["UNK"]);

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    /// Text content of the token.
    pub text: &'a str,

    /// Starting position of the token in bytes.
    pub byte_start: usize,

    /// Ending position of the token in bytes.
    pub byte_end: usize,

    /// Position, expressed in number of tokens.
    pub position: usize,

    /// The length expressed in term of number of original tokens.
    pub position_length: usize,

    /// The ID of the word and a flag to indicate whether the word is registered in the dictionary.
    pub word_id: WordId,

    /// Reference of dictionary.
    pub dictionary: &'a Dictionary,

    /// Reference of user dictionary.
    pub user_dictionary: Option<&'a UserDictionary>,

    /// Detailes about the token.
    /// It contains metadata for tokens, such as part-of-speech information.
    details: Option<Vec<String>>,
}

impl<'a> Token<'a> {
    pub fn new(
        text: &'a str,
        start: usize,
        end: usize,
        position: usize,
        word_id: WordId,
        dictionary: &'a Dictionary,
        user_dictionary: Option<&'a UserDictionary>,
    ) -> Self {
        Self {
            text,
            details: None,
            byte_start: start,
            byte_end: end,
            position,
            position_length: 1,
            word_id,
            dictionary,
            user_dictionary,
        }
    }

    fn details(&self) -> Option<Vec<&str>> {
        match &self.details {
            Some(details) => {
                let mut v = Vec::new();
                for detail in details.iter() {
                    let a = detail.as_str();
                    v.push(a);
                }
                Some(v)
            }
            None => None,
        }
    }

    // pub fn get_details(&mut self) -> Option<Vec<String>> {
    pub fn get_details(&mut self) -> Option<Vec<&str>> {
        if self.details.is_some() {
            return self.details();
        }

        if self.word_id.is_unknown() {
            self.set_details(Some(UNK.iter().map(|v| v.to_string()).collect()));
            return self.details();
        }

        let (words_idx_data, words_data) = if self.word_id.is_system() {
            (
                &*self.dictionary.words_idx_data,
                &*self.dictionary.words_data,
            )
        } else {
            match self.user_dictionary {
                Some(user_dictionary) => (
                    user_dictionary.words_idx_data.as_slice(),
                    user_dictionary.words_data.as_slice(),
                ),
                None => return None,
            }
        };

        let idx = LittleEndian::read_u32(&words_idx_data[4 * self.word_id.0 as usize..][..4]);
        let data = &words_data[idx as usize..];

        self.details = match bincode::deserialize_from(data) {
            Ok(details) => Some(details),
            Err(_err) => None,
        };

        self.details()
    }

    pub fn set_details(&mut self, details: Option<Vec<String>>) -> &Token<'a> {
        self.details = details;
        self
    }
}

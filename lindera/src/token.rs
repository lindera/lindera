use std::borrow::Cow;

use serde::Serialize;

use lindera_core::dictionary::word_entry::WordId;
use lindera_core::dictionary::{Dictionary, UserDictionary, UNK};

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    /// Text content of the token.
    pub text: Cow<'a, str>,

    /// Starting position of the token in bytes.
    pub byte_start: usize,

    /// Ending position of the token in bytes.
    pub byte_end: usize,

    /// Position, expressed in number of tokens.
    pub position: usize,

    /// The length expressed in terms of number of original tokens.
    pub position_length: usize,

    /// The ID of the word and a flag to indicate whether the word is registered in the dictionary.
    pub word_id: WordId,

    /// Reference of dictionary.
    pub dictionary: &'a Dictionary,

    /// Reference of user dictionary.
    pub user_dictionary: Option<&'a UserDictionary>,

    pub details: Option<Vec<Cow<'a, str>>>,
}

impl<'a> Token<'a> {
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

            self.details = Some(tmp.into_iter().map(|x| Cow::Borrowed(x)).collect());
        }

        // convert Cow to &str.
        self.details
            .as_ref()
            .map(|vec| vec.iter().map(|x| x.as_ref()).collect())
            .unwrap_or_else(|| UNK.to_vec())
    }

    pub fn get_detail(&mut self, index: usize) -> Option<&str> {
        self.details().get(index).map(|x| *x).or_else(|| None)
    }

    pub fn set_detail(&mut self, index: usize, detail: Cow<'a, str>) {
        if let Some(details) = self.details.as_mut() {
            details[index] = detail;
        }
    }
}

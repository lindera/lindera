use std::borrow::Cow;

use byteorder::{ByteOrder, LittleEndian};
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::{dictionary::Dictionary, user_dictionary::UserDictionary, word_entry::WordId};

static UNK: Lazy<Vec<&str>> = Lazy::new(|| vec!["UNK"]);

#[derive(Serialize, Clone)]
pub struct Token<'a> {
    text: Cow<'a, str>,
    details: Option<Vec<String>>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub word_id: WordId,
    pub dictionary: &'a Dictionary,
    pub user_dictionary: Option<&'a UserDictionary>,
}

impl<'a> Token<'a> {
    pub fn new(
        text: &str,
        start: usize,
        end: usize,
        word_id: WordId,
        dictionary: &'a Dictionary,
        user_dictionary: Option<&'a UserDictionary>,
    ) -> Self {
        Self {
            text: Cow::Owned(text.to_string()),
            details: None,
            byte_start: start,
            byte_end: end,
            word_id,
            dictionary,
            user_dictionary,
        }
    }

    pub fn get_text(&self) -> &str {
        self.text.as_ref()
    }

    // pub fn set_text(&mut self, text: &str) -> &Token<'a> {
    pub fn set_text(&mut self, text: String) -> &Token<'a> {
        self.text = Cow::Owned(text);
        self
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
                self.dictionary.words_idx_data.as_slice(),
                self.dictionary.words_data.as_slice(),
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

#[cfg(test)]
mod tests {}

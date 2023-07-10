pub mod japanese_base_form;
pub mod japanese_compound_word;
pub mod japanese_kana;
pub mod japanese_katakana_stem;
pub mod japanese_keep_tags;
pub mod japanese_number;
pub mod japanese_reading_form;
pub mod japanese_stop_tags;
pub mod keep_words;
pub mod korean_keep_tags;
pub mod korean_reading_form;
pub mod korean_stop_tags;
pub mod length;
pub mod lowercase;
pub mod mapping;
pub mod stop_words;
pub mod uppercase;

use std::ops::Deref;

use lindera_core::LinderaResult;
// use lindera_tokenizer::token::Token;

use crate::token::Token;

pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn name(&self) -> &str;
    fn apply<'a>(&self, tokens: &mut Vec<Token>) -> LinderaResult<()>;
}

pub struct BoxTokenFilter(Box<dyn TokenFilter + 'static + Send + Sync>);

impl Deref for BoxTokenFilter {
    type Target = dyn TokenFilter;

    fn deref(&self) -> &dyn TokenFilter {
        &*self.0
    }
}

impl<T: TokenFilter> From<T> for BoxTokenFilter {
    fn from(token_filter: T) -> BoxTokenFilter {
        BoxTokenFilter(Box::new(token_filter))
    }
}

pub trait TokenFilterClone {
    fn box_clone(&self) -> BoxTokenFilter;
}

impl<T: TokenFilter + Clone + 'static> TokenFilterClone for T {
    fn box_clone(&self) -> BoxTokenFilter {
        BoxTokenFilter::from(self.clone())
    }
}

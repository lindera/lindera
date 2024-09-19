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

use serde_json::Value;
use std::ops::Deref;

use crate::parse_cli_flag;
use crate::token::Token;
use crate::token_filter::japanese_base_form::{
    JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
    JAPANESE_BASE_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_compound_word::{
    JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
    JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_kana::{
    JapaneseKanaTokenFilter, JapaneseKanaTokenFilterConfig, JAPANESE_KANA_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_katakana_stem::{
    JapaneseKatakanaStemTokenFilter, JapaneseKatakanaStemTokenFilterConfig,
    JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_keep_tags::{
    JapaneseKeepTagsTokenFilter, JapaneseKeepTagsTokenFilterConfig,
    JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_number::{
    JapaneseNumberTokenFilter, JapaneseNumberTokenFilterConfig, JAPANESE_NUMBER_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_reading_form::{
    JapaneseReadingFormTokenFilter, JapaneseReadingFormTokenFilterConfig,
    JAPANESE_READING_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::japanese_stop_tags::{
    JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig,
    JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::keep_words::{
    KeepWordsTokenFilter, KeepWordsTokenFilterConfig, KEEP_WORDS_TOKEN_FILTER_NAME,
};
use crate::token_filter::korean_keep_tags::{
    KoreanKeepTagsTokenFilter, KoreanKeepTagsTokenFilterConfig, KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::korean_reading_form::{
    KoreanReadingFormTokenFilter, KOREAN_READING_FORM_TOKEN_FILTER_NAME,
};
use crate::token_filter::korean_stop_tags::{
    KoreanStopTagsTokenFilter, KoreanStopTagsTokenFilterConfig, KOREAN_STOP_TAGS_TOKEN_FILTER_NAME,
};
use crate::token_filter::length::{
    LengthTokenFilter, LengthTokenFilterConfig, LENGTH_TOKEN_FILTER_NAME,
};
use crate::token_filter::lowercase::{LowercaseTokenFilter, LOWERCASE_TOKEN_FILTER_NAME};
use crate::token_filter::mapping::{
    MappingTokenFilter, MappingTokenFilterConfig, MAPPING_TOKEN_FILTER_NAME,
};
use crate::token_filter::stop_words::{
    StopWordsTokenFilter, StopWordsTokenFilterConfig, STOP_WORDS_TOKEN_FILTER_NAME,
};
use crate::token_filter::uppercase::{UppercaseTokenFilter, UPPERCASE_TOKEN_FILTER_NAME};
use crate::{LinderaErrorKind, LinderaResult};

pub trait TokenFilter: 'static + Send + Sync + TokenFilterClone {
    fn name(&self) -> &'static str;
    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()>;
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

pub struct TokenFilterLoader {}

impl TokenFilterLoader {
    pub fn load_from_value(kind: &str, value: &Value) -> LinderaResult<BoxTokenFilter> {
        let token_filter = match kind {
            JAPANESE_BASE_FORM_TOKEN_FILTER_NAME => {
                let config = JapaneseBaseFormTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseBaseFormTokenFilter::new(config))
            }
            JAPANESE_COMPOUND_WORD_TOKEN_FILTER_NAME => {
                let config = JapaneseCompoundWordTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseCompoundWordTokenFilter::new(config))
            }
            JAPANESE_KANA_TOKEN_FILTER_NAME => {
                let config = JapaneseKanaTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseKanaTokenFilter::new(config))
            }
            JAPANESE_KATAKANA_STEM_TOKEN_FILTER_NAME => {
                let config = JapaneseKatakanaStemTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseKatakanaStemTokenFilter::new(config))
            }
            JAPANESE_KEEP_TAGS_TOKEN_FILTER_NAME => {
                let config = JapaneseKeepTagsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseKeepTagsTokenFilter::new(config))
            }
            JAPANESE_NUMBER_TOKEN_FILTER_NAME => {
                let config = JapaneseNumberTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseNumberTokenFilter::new(config))
            }
            JAPANESE_READING_FORM_TOKEN_FILTER_NAME => {
                let config = JapaneseReadingFormTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseReadingFormTokenFilter::new(config))
            }
            JAPANESE_STOP_TAGS_TOKEN_FILTER_NAME => {
                let config = JapaneseStopTagsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(JapaneseStopTagsTokenFilter::new(config))
            }
            KEEP_WORDS_TOKEN_FILTER_NAME => {
                let config = KeepWordsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(KeepWordsTokenFilter::new(config))
            }
            KOREAN_KEEP_TAGS_TOKEN_FILTER_NAME => {
                let config = KoreanKeepTagsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(KoreanKeepTagsTokenFilter::new(config))
            }
            KOREAN_READING_FORM_TOKEN_FILTER_NAME => {
                BoxTokenFilter::from(KoreanReadingFormTokenFilter::new())
            }
            KOREAN_STOP_TAGS_TOKEN_FILTER_NAME => {
                let config = KoreanStopTagsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(KoreanStopTagsTokenFilter::new(config))
            }
            LENGTH_TOKEN_FILTER_NAME => {
                let config = LengthTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(LengthTokenFilter::new(config))
            }
            LOWERCASE_TOKEN_FILTER_NAME => BoxTokenFilter::from(LowercaseTokenFilter::new()),
            MAPPING_TOKEN_FILTER_NAME => {
                let config = MappingTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(MappingTokenFilter::new(config)?)
            }
            STOP_WORDS_TOKEN_FILTER_NAME => {
                let config = StopWordsTokenFilterConfig::from_value(value)?;
                BoxTokenFilter::from(StopWordsTokenFilter::new(config))
            }
            UPPERCASE_TOKEN_FILTER_NAME => BoxTokenFilter::from(UppercaseTokenFilter::new()),
            _ => {
                return Err(LinderaErrorKind::Deserialize
                    .with_error(anyhow::anyhow!("unsupported token filter: {}", kind)));
            }
        };

        Ok(token_filter)
    }

    pub fn load_from_cli_flag(cli_flag: &str) -> LinderaResult<BoxTokenFilter> {
        let (kind, args) = parse_cli_flag(cli_flag)?;

        let character_filter = Self::load_from_value(kind, &args)?;

        Ok(character_filter)
    }
}

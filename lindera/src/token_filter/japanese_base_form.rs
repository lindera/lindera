use serde::{Deserialize, Serialize};

use lindera_core::token_filter::TokenFilter;

use crate::{error::LinderaErrorKind, DictionaryKind, LinderaResult, Token};

pub const JAPANESE_BASE_FORM_TOKEN_FILTER_NAME: &str = "japanese_base_form";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct JapaneseBaseFormTokenFilterConfig {
    kind: DictionaryKind,
}

impl JapaneseBaseFormTokenFilterConfig {
    pub fn new(kind: DictionaryKind) -> Self {
        Self { kind }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        serde_json::from_slice::<JapaneseBaseFormTokenFilterConfig>(data)
            .map_err(|err| LinderaErrorKind::Deserialize.with_error(err))
    }
}

/// Replace the term text with the base form registered in the morphological dictionary.
/// This acts as a lemmatizer for verbs and adjectives.
///
#[derive(Clone, Debug)]
pub struct JapaneseBaseFormTokenFilter {
    config: JapaneseBaseFormTokenFilterConfig,
}

impl JapaneseBaseFormTokenFilter {
    pub fn new(config: JapaneseBaseFormTokenFilterConfig) -> Self {
        Self { config }
    }

    pub fn from_slice(data: &[u8]) -> LinderaResult<Self> {
        Ok(Self::new(JapaneseBaseFormTokenFilterConfig::from_slice(
            data,
        )?))
    }
}

impl TokenFilter for JapaneseBaseFormTokenFilter {
    fn name(&self) -> &'static str {
        JAPANESE_BASE_FORM_TOKEN_FILTER_NAME
    }

    fn apply<'a>(&self, tokens: &mut Vec<Token<'a>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let details = match token.get_details() {
                Some(details) => details,
                None => {
                    return Err(LinderaErrorKind::Io
                        .with_error(anyhow::anyhow!("Failed to get word details.")))
                }
            };

            if details[0] == "UNK" {
                // NOOP
                continue;
            }
            match self.config.kind {
                #[cfg(feature = "ipadic")]
                DictionaryKind::IPADIC => {
                    let new_text = details[6].to_string();
                    token.set_text(new_text);
                }
                #[cfg(feature = "unidic")]
                DictionaryKind::UniDic => {
                    let new_text = details[10].to_string();
                    token.set_text(new_text);
                }
                _ => {
                    // NOOP
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use lindera_core::{token_filter::TokenFilter, word_entry::WordId};

    #[cfg(any(feature = "ipadic", feature = "unidic",))]
    use crate::{
        builder,
        token_filter::japanese_base_form::{
            JapaneseBaseFormTokenFilter, JapaneseBaseFormTokenFilterConfig,
        },
        DictionaryKind, Token,
    };

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_config_from_slice_ipadic() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let config = JapaneseBaseFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::IPADIC);
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_config_from_slice_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let config = JapaneseBaseFormTokenFilterConfig::from_slice(config_str.as_bytes()).unwrap();

        assert_eq!(config.kind, DictionaryKind::UniDic);
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_from_slice_ipadic() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let result = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_from_slice_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let result = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes());

        assert_eq!(true, result.is_ok());
    }

    #[cfg(feature = "ipadic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply_ipadic() {
        let config_str = r#"
        {
            "kind": "ipadic"
        }
        "#;
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田空港", 0, 12, 0, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "羽田空港".to_string(),
                    "ハネダクウコウ".to_string(),
                    "ハネダクーコー".to_string(),
                ]))
                .clone(),
            Token::new("に", 12, 15, 1, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助詞".to_string(),
                    "格助詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "に".to_string(),
                    "ニ".to_string(),
                    "ニ".to_string(),
                ]))
                .clone(),
            Token::new("あり", 15, 18, 2, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "動詞".to_string(),
                    "自立".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "五段・ラ行".to_string(),
                    "連用形".to_string(),
                    "ある".to_string(),
                    "アリ".to_string(),
                    "アリ".to_string(),
                ]))
                .clone(),
            Token::new("ます", 18, 24, 3, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助動詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "特殊・マス".to_string(),
                    "基本形".to_string(),
                    "ます".to_string(),
                    "マス".to_string(),
                    "マス".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].get_text(), "羽田空港");
        assert_eq!(tokens[1].get_text(), "に");
        assert_eq!(tokens[2].get_text(), "ある");
        assert_eq!(tokens[3].get_text(), "ます");
    }

    #[cfg(feature = "unidic")]
    #[test]
    fn test_japanese_base_form_token_filter_apply_unidic() {
        let config_str = r#"
        {
            "kind": "unidic"
        }
        "#;
        let filter = JapaneseBaseFormTokenFilter::from_slice(config_str.as_bytes()).unwrap();

        let dictionary = builder::load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new("羽田", 0, 6, 0, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "固有名詞".to_string(),
                    "人名".to_string(),
                    "姓".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "ハタ".to_string(),
                    "ハタ".to_string(),
                    "羽田".to_string(),
                    "ハタ".to_string(),
                    "羽田".to_string(),
                    "ハタ".to_string(),
                    "固".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("空港", 6, 12, 1, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "名詞".to_string(),
                    "普通名詞".to_string(),
                    "一般".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "クウコウ".to_string(),
                    "空港".to_string(),
                    "空港".to_string(),
                    "クーコー".to_string(),
                    "空港".to_string(),
                    "クーコー".to_string(),
                    "漢".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("に", 12, 15, 2, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助詞".to_string(),
                    "格助詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "ニ".to_string(),
                    "に".to_string(),
                    "に".to_string(),
                    "ニ".to_string(),
                    "に".to_string(),
                    "ニ".to_string(),
                    "和".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("あり", 15, 18, 3, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "動詞".to_string(),
                    "非自立可能".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "五段-ラ行".to_string(),
                    "連用形-一般".to_string(),
                    "アル".to_string(),
                    "有る".to_string(),
                    "あり".to_string(),
                    "アリ".to_string(),
                    "ある".to_string(),
                    "アル".to_string(),
                    "和".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
            Token::new("ます", 18, 24, 4, WordId::default(), &dictionary, None)
                .set_details(Some(vec![
                    "助動詞".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "助動詞-マス".to_string(),
                    "終止形-一般".to_string(),
                    "マス".to_string(),
                    "ます".to_string(),
                    "ます".to_string(),
                    "マス".to_string(),
                    "ます".to_string(),
                    "マス".to_string(),
                    "和".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                    "*".to_string(),
                ]))
                .clone(),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].get_text(), "羽田");
        assert_eq!(tokens[1].get_text(), "空港");
        assert_eq!(tokens[2].get_text(), "に");
        assert_eq!(tokens[3].get_text(), "ある");
        assert_eq!(tokens[4].get_text(), "ます");
    }
}

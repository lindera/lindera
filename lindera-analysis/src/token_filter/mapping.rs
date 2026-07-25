use std::borrow::Cow;
use std::collections::HashMap;

use daachorse::DoubleArrayAhoCorasick;
use daachorse::DoubleArrayAhoCorasickBuilder;
use daachorse::MatchKind;
use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera::LinderaResult;
use lindera::error::LinderaErrorKind;
use lindera::token::Token;

pub const MAPPING_TOKEN_FILTER_NAME: &str = "mapping";

pub type MappingTokenFilterConfig = Value;

/// Replace characters with the specified character mappings.
///
#[derive(Clone)]
pub struct MappingTokenFilter {
    mapping: HashMap<String, String>,
    trie: DoubleArrayAhoCorasick<u32>,
}

impl MappingTokenFilter {
    /// Create a new `MappingTokenFilter` from a surface-to-replacement mapping.
    ///
    /// # Arguments
    ///
    /// * `mapping` - A map from surface text to its replacement text. Keys must be
    ///   non-empty; an empty key would match at every byte position under the
    ///   leftmost-longest search strategy used by `apply`, which is never a
    ///   meaningful mapping and is therefore rejected.
    ///
    /// # Returns
    ///
    /// A `MappingTokenFilter`, or an error if `mapping` contains an empty key or
    /// the underlying Aho-Corasick automaton fails to build.
    pub fn new(mapping: HashMap<String, String>) -> LinderaResult<Self> {
        if mapping.keys().any(|key| key.is_empty()) {
            return Err(LinderaErrorKind::Args
                .with_error(anyhow::anyhow!("mapping key must not be empty.")));
        }

        let mut keyset: Vec<(&[u8], u32)> = Vec::new();
        let mut keys = mapping.keys().collect::<Vec<_>>();
        keys.sort();
        for (value, key) in keys.into_iter().enumerate() {
            keyset.push((key.as_bytes(), value as u32));
        }

        let trie = DoubleArrayAhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build_with_values(keyset)
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?;

        Ok(Self { mapping, trie })
    }

    pub fn from_config(config: &MappingTokenFilterConfig) -> LinderaResult<Self> {
        let mapping = config
            .get("mapping")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                LinderaErrorKind::Parse.with_error(anyhow::anyhow!("mapping must be an object."))
            })?
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect::<HashMap<String, String>>();

        Self::new(mapping)
    }
}

impl TokenFilter for MappingTokenFilter {
    fn name(&self) -> &'static str {
        MAPPING_TOKEN_FILTER_NAME
    }

    /// Apply the filter to each token's surface, in place.
    ///
    /// Performs a single leftmost-longest pass over each token's surface with the
    /// underlying Aho-Corasick automaton: at each position the longest configured
    /// key wins, matches never overlap, and scanning resumes immediately after
    /// each match.
    ///
    /// # Arguments
    ///
    /// * `tokens` - The tokens to filter; each token's `surface` is replaced in
    ///   place with the mapped text.
    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            let mut result = String::with_capacity(token.surface.len());

            {
                let source = token.surface.as_ref();
                let mut cursor = 0_usize;

                for m in self.trie.leftmost_find_iter(source) {
                    // Keys are validated non-empty in `new`, and all keys are
                    // valid UTF-8, so matches are non-empty, non-overlapping,
                    // strictly ascending, and always land on char boundaries.
                    debug_assert!(m.start() >= cursor && m.end() > m.start());

                    result.push_str(&source[cursor..m.start()]);
                    result.push_str(&self.mapping[&source[m.start()..m.end()]]);
                    cursor = m.end();
                }

                result.push_str(&source[cursor..]);
            }

            token.surface = Cow::Owned(result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::token_filter::mapping::{MappingTokenFilter, MappingTokenFilterConfig};

    #[test]
    fn test_mapping_token_filter_empty_key_rejected() {
        let mut mapping = HashMap::new();
        mapping.insert(String::new(), "x".to_string());
        assert!(MappingTokenFilter::new(mapping).is_err());

        let mut mapping = HashMap::new();
        mapping.insert("a".to_string(), "b".to_string());
        assert!(MappingTokenFilter::new(mapping).is_ok());
    }

    #[test]
    fn test_mapping_token_filter_config() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let result: Result<MappingTokenFilterConfig, _> = serde_json::from_str(config_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mapping_token_filter() {
        let config_str = r#"
        {
            "mapping": {
                "ｱ": "ア",
                "ｲ": "イ",
                "ｳ": "ウ",
                "ｴ": "エ",
                "ｵ": "オ"
            }
        }
        "#;
        let config = serde_json::from_str::<MappingTokenFilterConfig>(config_str).unwrap();

        let result = MappingTokenFilter::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_mapping_token_filter_apply_ipadic() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
        {
            "mapping": {
                "籠": "篭"
            }
        }
        "#;
        let config = serde_json::from_str::<MappingTokenFilterConfig>(config_str).unwrap();

        let filter = MappingTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token {
                surface: Cow::Borrowed("籠原"),
                byte_start: 0,
                byte_end: 6,
                position: 0,
                position_length: 1,
                word_id: WordId::new(LexType::System, 312630),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("固有名詞"),
                    Cow::Borrowed("一般"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("籠原"),
                    Cow::Borrowed("カゴハラ"),
                    Cow::Borrowed("カゴハラ"),
                ]),
            },
            Token {
                surface: Cow::Borrowed("駅"),
                byte_start: 6,
                byte_end: 9,
                position: 1,
                position_length: 1,
                word_id: WordId::new(LexType::System, 383791),
                dictionary: &dictionary,
                user_dictionary: None,
                details: Some(vec![
                    Cow::Borrowed("名詞"),
                    Cow::Borrowed("接尾"),
                    Cow::Borrowed("地域"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("*"),
                    Cow::Borrowed("駅"),
                    Cow::Borrowed("エキ"),
                    Cow::Borrowed("エキ"),
                ]),
            },
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].surface, "篭原");
        assert_eq!(&tokens[1].surface, "駅");
    }

    #[test]
    #[cfg(feature = "embed-ipadic")]
    fn test_mapping_token_filter_apply_longest_match() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use lindera::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera::token::Token;
        use lindera_dictionary::viterbi::LexType;

        let config_str = r#"
        {
            "mapping": {
                "ab": "X",
                "abc": "YY"
            }
        }
        "#;
        let config = serde_json::from_str::<MappingTokenFilterConfig>(config_str).unwrap();

        let filter = MappingTokenFilter::from_config(&config).unwrap();

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();

        let mut tokens: Vec<Token> = vec![Token {
            surface: Cow::Borrowed("abcab"),
            byte_start: 0,
            byte_end: 5,
            position: 0,
            position_length: 1,
            word_id: WordId::new(LexType::System, 0),
            dictionary: &dictionary,
            user_dictionary: None,
            details: None,
        }];

        filter.apply(&mut tokens).unwrap();

        // "abc" (longest match at 0) wins over "ab"; "ab" (longest at 3) wins.
        assert_eq!(tokens.len(), 1);
        assert_eq!(&tokens[0].surface, "YYX");
    }
}

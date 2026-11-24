use std::borrow::Cow;

use serde_json::Value;

use crate::token_filter::TokenFilter;
use lindera_segmenter::LinderaResult;
use lindera_segmenter::token::Token;

pub const KOREAN_READING_FORM_TOKEN_FILTER_NAME: &str = "korean_reading_form";

pub type KoreanReadingFormTokenFilterConfig = Value;

/// Replace the text of a token with the reading of the text as registered in the morphological dictionary.
///
#[derive(Clone, Debug)]
pub struct KoreanReadingFormTokenFilter {}

impl KoreanReadingFormTokenFilter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn from_config(_config: &KoreanReadingFormTokenFilterConfig) -> LinderaResult<Self> {
        Ok(Self::new())
    }
}

impl Default for KoreanReadingFormTokenFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for KoreanReadingFormTokenFilter {
    fn name(&self) -> &'static str {
        KOREAN_READING_FORM_TOKEN_FILTER_NAME
    }

    fn apply(&self, tokens: &mut Vec<Token<'_>>) -> LinderaResult<()> {
        for token in tokens.iter_mut() {
            // Skip unknown tokens. Their details come from `unk.def`, so the
            // first one is a real part of speech rather than the `UNK` sentinel
            // this used to test for.
            if token.word_id.is_unknown() {
                continue;
            }

            // `*` is the MeCab placeholder for an absent field, and `details` is
            // padded with it to the schema width, so an entry that carries no
            // reading yields `Some("*")` rather than `None`.
            if let Some(reading) = token.get("reading")
                && reading != "*"
            {
                token.surface = Cow::Owned(reading.to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_reading_form_token_filter_apply() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let filter = KoreanReadingFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

        let mut tokens: Vec<Token> = vec![
            Token::new(
                Cow::Borrowed("한국어"),
                0,
                9,
                0,
                WordId::new(LexType::System, 770060),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("NNG"),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed("한국어"),
                Cow::Borrowed("Compound"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("한국/NNG/*+어/NNG/*"),
            ]),
            Token::new(
                Cow::Borrowed("의"),
                9,
                12,
                1,
                WordId::new(LexType::System, 576336),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("JKG"),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed("의"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
            Token::new(
                Cow::Borrowed("형태소"),
                12,
                21,
                2,
                WordId::new(LexType::System, 787807),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("NNG"),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed("형태소"),
                Cow::Borrowed("Compound"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("형태/NNG/*+소/NNG/*"),
            ]),
            Token::new(
                Cow::Borrowed("분석"),
                21,
                27,
                3,
                WordId::new(LexType::System, 383955),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("NNG"),
                Cow::Borrowed("행위"),
                Cow::Borrowed("T"),
                Cow::Borrowed("분석"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
            Token::new(
                Cow::Borrowed("을"),
                27,
                30,
                4,
                WordId::new(LexType::System, 574939),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("JKO"),
                Cow::Borrowed("*"),
                Cow::Borrowed("T"),
                Cow::Borrowed("을"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
            Token::new(
                Cow::Borrowed("할"),
                30,
                33,
                5,
                WordId::new(LexType::System, 774117),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("VV+ETM"),
                Cow::Borrowed("*"),
                Cow::Borrowed("T"),
                Cow::Borrowed("할"),
                Cow::Borrowed("Inflect"),
                Cow::Borrowed("VV"),
                Cow::Borrowed("ETM"),
                Cow::Borrowed("하/VV/*+ᆯ/ETM/*"),
            ]),
            Token::new(
                Cow::Borrowed("수"),
                33,
                36,
                6,
                WordId::new(LexType::System, 444151),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("NNG"),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed("수"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
            Token::new(
                Cow::Borrowed("있"),
                36,
                39,
                7,
                WordId::new(LexType::System, 602850),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("VX"),
                Cow::Borrowed("*"),
                Cow::Borrowed("T"),
                Cow::Borrowed("있"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
            Token::new(
                Cow::Borrowed("습니다"),
                39,
                48,
                8,
                WordId::new(LexType::System, 458024),
                &dictionary,
                None,
            )
            .with_details(vec![
                Cow::Borrowed("EF"),
                Cow::Borrowed("*"),
                Cow::Borrowed("F"),
                Cow::Borrowed("습니다"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
                Cow::Borrowed("*"),
            ]),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 9);
        assert_eq!(&tokens[0].surface, "한국어");
        assert_eq!(&tokens[1].surface, "의");
        assert_eq!(&tokens[2].surface, "형태소");
        assert_eq!(&tokens[3].surface, "분석");
        assert_eq!(&tokens[4].surface, "을");
        assert_eq!(&tokens[5].surface, "할");
        assert_eq!(&tokens[6].surface, "수");
        assert_eq!(&tokens[7].surface, "있");
        assert_eq!(&tokens[8].surface, "습니다");
    }

    // A word that is not in the dictionary takes its details from `unk.def`, so
    // its part of speech is a real one and its reading is the placeholder `*`.
    // Neither may be substituted into the surface form.
    #[test]
    #[cfg(feature = "embed-ko-dic")]
    fn test_korean_reading_form_token_filter_keeps_surface_without_reading() {
        use std::borrow::Cow;

        use crate::token_filter::TokenFilter;
        use crate::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
        use lindera_dictionary::viterbi::LexType;
        use lindera_segmenter::dictionary::{DictionaryKind, WordId, load_embedded_dictionary};
        use lindera_segmenter::token::Token;

        let filter = KoreanReadingFormTokenFilter::new();

        let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();

        // The details an out-of-vocabulary word receives from `unk.def`: a real
        // part of speech, then `*` for every remaining field.
        let unknown_details = vec![
            Cow::Borrowed("SY"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
            Cow::Borrowed("*"),
        ];

        let mut tokens: Vec<Token> = vec![
            // Out of vocabulary, so skipped by the unknown-word guard.
            Token::new(
                Cow::Borrowed("!"),
                0,
                1,
                0,
                WordId::new(LexType::Unknown, 3),
                &dictionary,
                None,
            )
            .with_details(unknown_details.clone()),
            // In the dictionary, but carries no reading, so skipped by the
            // placeholder guard.
            Token::new(
                Cow::Borrowed("."),
                1,
                2,
                1,
                WordId::new(LexType::System, 3),
                &dictionary,
                None,
            )
            .with_details(unknown_details),
        ];

        filter.apply(&mut tokens).unwrap();

        assert_eq!(tokens.len(), 2);
        assert_eq!(&tokens[0].surface, "!");
        assert_eq!(&tokens[1].surface, ".");
    }
}

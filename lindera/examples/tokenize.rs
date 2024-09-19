use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use std::collections::HashSet;

        use lindera::character_filter::japanese_iteration_mark::{
            JapaneseIterationMarkCharacterFilter, JapaneseIterationMarkCharacterFilterConfig,
        };
        use lindera::character_filter::unicode_normalize::{
            UnicodeNormalizeCharacterFilter, UnicodeNormalizeCharacterFilterConfig,
            UnicodeNormalizeKind,
        };
        use lindera::character_filter::BoxCharacterFilter;
        use lindera::core::mode::Mode;
        use lindera::dictionary::{DictionaryConfig, DictionaryKind};
        use lindera::segmenter::{Segmenter, SegmenterConfig};
        use lindera::token_filter::japanese_compound_word::{
            JapaneseCompoundWordTokenFilter, JapaneseCompoundWordTokenFilterConfig,
        };
        use lindera::token_filter::japanese_number::{
            JapaneseNumberTokenFilter, JapaneseNumberTokenFilterConfig,
        };
        use lindera::token_filter::japanese_stop_tags::{
            JapaneseStopTagsTokenFilter, JapaneseStopTagsTokenFilterConfig,
        };
        use lindera::token_filter::BoxTokenFilter;
        use lindera::tokenizer::Tokenizer;

        let mut character_filters: Vec<BoxCharacterFilter> = Vec::new();

        let unicode_normalize_character_filter_config =
            UnicodeNormalizeCharacterFilterConfig::new(UnicodeNormalizeKind::NFKC);
        let unicode_normalize_character_filter =
            UnicodeNormalizeCharacterFilter::new(unicode_normalize_character_filter_config);
        character_filters.push(BoxCharacterFilter::from(unicode_normalize_character_filter));

        let japanese_iteration_mark_character_filter_config =
            JapaneseIterationMarkCharacterFilterConfig::new(true, true);
        let japanese_iteration_mark_character_filter = JapaneseIterationMarkCharacterFilter::new(
            japanese_iteration_mark_character_filter_config,
        );
        character_filters.push(BoxCharacterFilter::from(
            japanese_iteration_mark_character_filter,
        ));

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        let config = SegmenterConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        let segmenter = Segmenter::from_config(config).unwrap();

        let mut token_filters: Vec<BoxTokenFilter> = Vec::new();

        let japanese_compound_word_token_filter_config =
            JapaneseCompoundWordTokenFilterConfig::new(
                DictionaryKind::IPADIC,
                HashSet::from_iter(vec!["名詞,数".to_string()]),
                Some("名詞,数".to_string()),
            )?;
        let japanese_compound_word_token_filter =
            JapaneseCompoundWordTokenFilter::new(japanese_compound_word_token_filter_config);
        token_filters.push(BoxTokenFilter::from(japanese_compound_word_token_filter));

        let japanese_number_token_filter_config =
            JapaneseNumberTokenFilterConfig::new(Some(HashSet::from_iter(vec![
                "名詞,数".to_string()
            ])));
        let japanese_number_token_filter =
            JapaneseNumberTokenFilter::new(japanese_number_token_filter_config);
        token_filters.push(BoxTokenFilter::from(japanese_number_token_filter));

        let japanese_stop_tags_token_filter_config =
            JapaneseStopTagsTokenFilterConfig::new(HashSet::from_iter(vec![
                "接続詞".to_string(),
                "助詞".to_string(),
                "助詞,格助詞".to_string(),
                "助詞,格助詞,一般".to_string(),
                "助詞,格助詞,引用".to_string(),
                "助詞,格助詞,連語".to_string(),
                "助詞,係助詞".to_string(),
                "助詞,副助詞".to_string(),
                "助詞,間投助詞".to_string(),
                "助詞,並立助詞".to_string(),
                "助詞,終助詞".to_string(),
                "助詞,副助詞／並立助詞／終助詞".to_string(),
                "助詞,連体化".to_string(),
                "助詞,副詞化".to_string(),
                "助詞,特殊".to_string(),
                "助動詞".to_string(),
                "記号".to_string(),
                "記号,一般".to_string(),
                "記号,読点".to_string(),
                "記号,句点".to_string(),
                "記号,空白".to_string(),
                "記号,括弧閉".to_string(),
                "その他,間投".to_string(),
                "フィラー".to_string(),
                "非言語音".to_string(),
            ]));
        let japanese_stop_tags_token_filter =
            JapaneseStopTagsTokenFilter::new(japanese_stop_tags_token_filter_config);
        token_filters.push(BoxTokenFilter::from(japanese_stop_tags_token_filter));

        let tokenizer = Tokenizer::new(character_filters, segmenter, token_filters);

        let mut text =
            "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
        println!("text: {}", text);

        let tokens = tokenizer.tokenize(&mut text)?;

        for token in tokens {
            println!(
                "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
                token.text, token.byte_start, token.byte_end, token.details
            );
        }
    }

    Ok(())
}

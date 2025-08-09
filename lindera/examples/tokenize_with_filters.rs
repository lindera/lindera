use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use lindera::character_filter::BoxCharacterFilter;
        use lindera::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
        use lindera::character_filter::unicode_normalize::{
            UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
        };
        use lindera::dictionary::{DictionaryKind, load_embedded_dictionary};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::token_filter::BoxTokenFilter;
        use lindera::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
        use lindera::token_filter::japanese_number::JapaneseNumberTokenFilter;
        use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC)?;
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        let unicode_normalize_char_filter =
            UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

        let japanese_iterration_mark_char_filter =
            JapaneseIterationMarkCharacterFilter::new(true, true);

        let japanese_compound_word_token_filter = JapaneseCompoundWordTokenFilter::new(
            DictionaryKind::IPADIC,
            vec!["名詞,数".to_string(), "名詞,接尾,助数詞".to_string()]
                .into_iter()
                .collect(),
            Some("複合語".to_string()),
        );

        let japanese_number_token_filter =
            JapaneseNumberTokenFilter::new(Some(vec!["名詞,数".to_string()].into_iter().collect()));

        let japanese_stop_tags_token_filter = JapaneseStopTagsTokenFilter::new(
            vec![
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
            ]
            .into_iter()
            .collect(),
        );

        // Create a tokenizer.
        let mut tokenizer = Tokenizer::new(segmenter);

        tokenizer
            .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
            .append_character_filter(BoxCharacterFilter::from(
                japanese_iterration_mark_char_filter,
            ))
            .append_token_filter(BoxTokenFilter::from(japanese_compound_word_token_filter))
            .append_token_filter(BoxTokenFilter::from(japanese_number_token_filter))
            .append_token_filter(BoxTokenFilter::from(japanese_stop_tags_token_filter));

        // Tokenize a text.
        let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。";
        let tokens = tokenizer.tokenize(text)?;

        // Print the text and tokens.
        println!("text: {text}");
        for token in tokens {
            println!(
                "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
                token.text, token.byte_start, token.byte_end, token.details
            );
        }
    }

    Ok(())
}

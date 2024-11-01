use lindera::LinderaResult;
use serde_json::json;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);
        config_builder.append_character_filter("unicode_normalize", &json!({"kind": "nfkc"}));
        config_builder.append_character_filter(
            "japanese_iteration_mark",
            &json!({"normalize_kanji": true, "normalize_kana": true}),
        );
        config_builder.append_token_filter(
            "japanese_compound_word",
            &json!({
                "kind": "ipadic",
                "tags": [
                    "名詞,数",
                    "名詞,接尾,助数詞"
                ],
                "new_tag": "複合語"
            }),
        );
        config_builder.append_token_filter(
            "japanese_number",
            &json!({
                "tags": [
                    "名詞,数"
                ]
            }),
        );
        config_builder.append_token_filter(
            "japanese_stop_tags",
            &json!({
                "tags": [
                    "接続詞",
                    "助詞",
                    "助詞,格助詞",
                    "助詞,格助詞,一般",
                    "助詞,格助詞,引用",
                    "助詞,格助詞,連語",
                    "助詞,係助詞",
                    "助詞,副助詞",
                    "助詞,間投助詞",
                    "助詞,並立助詞",
                    "助詞,終助詞",
                    "助詞,副助詞／並立助詞／終助詞",
                    "助詞,連体化",
                    "助詞,副詞化",
                    "助詞,特殊",
                    "助動詞",
                    "記号",
                    "記号,一般",
                    "記号,読点",
                    "記号,句点",
                    "記号,空白",
                    "記号,括弧閉",
                    "その他,間投",
                    "フィラー",
                    "非言語音"
                ]
            }),
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build())?;

        // Tokenize a text.
        let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。";
        let tokens = tokenizer.tokenize(text)?;

        // Print the text and tokens.
        println!("text: {}", text);
        for token in tokens {
            println!(
                "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
                token.text, token.byte_start, token.byte_end, token.details
            );
        }
    }

    Ok(())
}

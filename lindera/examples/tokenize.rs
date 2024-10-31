use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::DictionaryKind;
        use lindera::mode::Mode;
        use lindera::tokenizer::{Tokenizer, TokenizerConfigBuilder};

        let mut config_builder = TokenizerConfigBuilder::new();
        config_builder.set_segmenter_dictionary_kind(&DictionaryKind::IPADIC);
        config_builder.set_segmenter_mode(&Mode::Normal);

        // Create a tokenizer.
        let tokenizer = Tokenizer::from_config(&config_builder.build())?;

        // Tokenize a text.
        let text = "関西国際空港限定トートバッグ";
        let mut tokens = tokenizer.tokenize(text)?;

        // Print the text and tokens.
        println!("text:\t{}", text);
        for token in tokens.iter_mut() {
            let details = token.details().join(",");
            println!("token:\t{}\t{}", token.text.as_ref(), details);
        }
    }

    Ok(())
}

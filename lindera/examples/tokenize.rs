use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use lindera::core::mode::Mode;
        use lindera::dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
        use lindera::tokenizer::Tokenizer;

        // Create a dictionary config.
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        // Load a dictionary from the dictionary config.
        let dictionary = load_dictionary_from_config(dictionary_config)?;

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(Mode::Normal, dictionary, None);

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

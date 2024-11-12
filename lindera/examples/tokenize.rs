use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use lindera::dictionary::{load_dictionary_from_kind, DictionaryKind};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC)?;
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            None, // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

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

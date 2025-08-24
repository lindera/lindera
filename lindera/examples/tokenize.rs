use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "embedded-ipadic")]
    {
        use lindera::dictionary::{DictionaryKind, load_embedded_dictionary};
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC)?;
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);

        let text = "関西国際空港限定トートバッグ";
        let mut tokens = tokenizer.tokenize(text)?;
        println!("text:\t{text}");
        for token in tokens.iter_mut() {
            let details = token.details().join(",");
            println!("token:\t{}\t{}", token.text.as_ref(), details);
        }
    }

    #[cfg(not(feature = "embedded-ipadic"))]
    {
        eprintln!(
            "This example requires the '{}' feature to be enabled.",
            "embedded-ipadic"
        );
        eprintln!(
            "Run with: cargo run --features {} --example tokenize",
            "embedded-ipadic"
        );
    }

    Ok(())
}

use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "embed-ipadic")]
    {
        use lindera::dictionary::load_dictionary;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let dictionary = load_dictionary("embedded://ipadic")?;
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);

        let text = "関西国際空港限定トートバッグ";
        let mut tokens = tokenizer.tokenize(text)?;
        println!("text:\t{text}");
        for token in tokens.iter_mut() {
            let details = token.details().join(",");
            println!("token:\t{}\t{}", token.surface.as_ref(), details);
        }
    }

    #[cfg(not(feature = "embed-ipadic"))]
    {
        eprintln!(
            "This example requires the '{}' feature to be enabled.",
            "embed-ipadic"
        );
        eprintln!(
            "Run with: cargo run --features {} --example tokenize",
            "embed-ipadic"
        );
    }

    Ok(())
}

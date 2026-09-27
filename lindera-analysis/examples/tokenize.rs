use lindera_segmenter::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "embed-ipadic")]
    {
        use lindera_analysis::tokenizer::Tokenizer;
        use lindera_segmenter::dictionary::load_dictionary;
        use lindera_segmenter::mode::Mode;
        use lindera_segmenter::segmenter::Segmenter;

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
        eprintln!("This example requires the 'embed-ipadic' feature to be enabled.");
        eprintln!("Run with: cargo run --features embed-ipadic --example tokenize");
    }

    Ok(())
}

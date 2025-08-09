use lindera::LinderaResult;

macro_rules! run_with_feature {
    ($feature:literal, $dict_kind:expr, $text:expr) => {
        #[cfg(feature = $feature)]
        {
            use lindera::dictionary::load_embedded_dictionary;
            use lindera::mode::Mode;
            use lindera::segmenter::Segmenter;
            use lindera::tokenizer::Tokenizer;

            let dictionary = load_embedded_dictionary($dict_kind)?;
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let tokenizer = Tokenizer::new(segmenter);

            let mut tokens = tokenizer.tokenize($text)?;
            println!("text:\t{}", $text);
            for token in tokens.iter_mut() {
                let details = token.details().join(",");
                println!("token:\t{}\t{}", token.text.as_ref(), details);
            }
        }

        #[cfg(not(feature = $feature))]
        {
            eprintln!(
                "This example requires the '{}' feature to be enabled.",
                $feature
            );
            eprintln!(
                "Run with: cargo run --features {} --example tokenize",
                $feature
            );
        }
    };
}

fn main() -> LinderaResult<()> {
    run_with_feature!(
        "ipadic",
        lindera::dictionary::DictionaryKind::IPADIC,
        "関西国際空港限定トートバッグ"
    );
    Ok(())
}

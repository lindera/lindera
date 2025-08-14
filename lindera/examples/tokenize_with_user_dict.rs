use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "embedded-ipadic")]
    {
        use std::path::PathBuf;

        use lindera::dictionary::{
            DictionaryKind, load_embedded_dictionary, load_user_dictionary_from_csv,
        };
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("ipadic_simple_userdic.csv");

        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC)?;
        let user_dictionary =
            load_user_dictionary_from_csv(DictionaryKind::IPADIC, user_dict_path.as_path())?;
        let segmenter = Segmenter::new(
            Mode::Normal,
            dictionary,
            Some(user_dictionary), // Assuming no user dictionary is provided
        );

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(segmenter);

        // Tokenize a text.
        let text = "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です";
        let mut tokens = tokenizer.tokenize(text)?;

        // Print the text and tokens.
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

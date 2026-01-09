use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "embed-ipadic")]
    {
        use std::fs::File;
        use std::path::PathBuf;

        use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};
        use lindera::error::LinderaErrorKind;
        use lindera::mode::Mode;
        use lindera::segmenter::Segmenter;
        use lindera::tokenizer::Tokenizer;

        let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("user_dict")
            .join("ipadic_simple_userdic.csv");

        let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../lindera-ipadic")
            .join("metadata.json");
        let metadata: Metadata = serde_json::from_reader(
            File::open(metadata_file)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
                .unwrap(),
        )
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
        .unwrap();

        let dictionary = load_dictionary("embedded://ipadic")?;
        let user_dictionary = load_user_dictionary(user_dict_path.to_str().unwrap(), &metadata)?;
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

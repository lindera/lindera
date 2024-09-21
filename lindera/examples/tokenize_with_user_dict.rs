use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::core::mode::Mode;
        use lindera::dictionary::{
            DictionaryConfig, DictionaryKind, DictionaryLoader, UserDictionaryConfig,
        };
        use lindera::tokenizer::Tokenizer;

        // Create a dictionary config.
        let dictionary_config = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        };

        // Load a dictionary from the dictionary config.
        let dictionary = DictionaryLoader::load_dictionary_from_config(dictionary_config)?;

        let user_dictionary_config = UserDictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: PathBuf::from("./resources/ipadic_simple_userdic.csv"),
        };

        let user_dictionary =
            DictionaryLoader::load_user_dictionary_from_config(user_dictionary_config)?;

        // Create a tokenizer.
        let tokenizer = Tokenizer::new(Mode::Normal, dictionary, Some(user_dictionary));

        // Tokenize a text.
        let text = "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です";
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

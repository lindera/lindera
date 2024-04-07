use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "cc-cedict")]
    {
        use std::path::PathBuf;

        use lindera::{
            DictionaryConfig, DictionaryKind, Mode, Tokenizer, TokenizerConfig,
            UserDictionaryConfig,
        };

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("cc-cedict_simple_userdic.csv"),
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let tokenizer = Tokenizer::from_config(config).unwrap();

        // tokenize the text
        let tokens = tokenizer.tokenize("羽田机场限定托特包。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

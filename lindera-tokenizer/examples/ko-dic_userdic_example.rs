use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ko-dic")]
    {
        use std::path::PathBuf;

        use lindera_core::viterbi::Mode;
        use lindera_dictionary::{DictionaryConfig, DictionaryKind, UserDictionaryConfig};
        use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("ko-dic_simple_userdic.csv"),
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: user_dictionary,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let tokenizer = Tokenizer::from_config(config).unwrap();

        // tokenize the text
        let tokens = tokenizer.tokenize("하네다공항한정토트백.")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

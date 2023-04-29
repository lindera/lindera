use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ko-dic")]
    {
        use lindera_core::mode::Mode;
        use lindera_dictionary::{DictionaryConfig, DictionaryKind};
        use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
        };

        #[allow(unused_variables)]
        let tokenizer = Tokenizer::from_config(config).unwrap();

        // tokenize the text
        let tokens = tokenizer.tokenize("한국어의형태해석을실시할수있습니다.")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

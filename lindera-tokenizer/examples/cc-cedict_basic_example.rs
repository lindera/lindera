use lindera_core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "cc-cedict")]
    {
        use lindera_core::viterbi::Mode;
        use lindera_dictionary::{DictionaryConfig, DictionaryKind};
        use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig};

        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::CcCedict),
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
        let tokens = tokenizer.tokenize("可以进行中文形态学分析。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

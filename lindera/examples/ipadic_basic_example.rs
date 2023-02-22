use lindera::LinderaResult;
#[cfg(feature = "ipadic")]
use lindera::{
    dictionary::DictionaryConfig,
    mode::Mode,
    tokenizer::{Tokenizer, TokenizerConfig},
    DictionaryKind,
};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
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
        let tokens = tokenizer.tokenize("日本語の形態素解析を行うことができます。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

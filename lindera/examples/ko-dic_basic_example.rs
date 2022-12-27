use lindera::LinderaResult;
#[cfg(feature = "ko-dic")]
use lindera::{
    mode::Mode,
    tokenizer::{DictionaryConfig, Tokenizer, TokenizerConfig},
    DictionaryKind,
};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ko-dic")]
    {
        let dictionary = DictionaryConfig {
            kind: Some(DictionaryKind::KoDic),
            path: None,
        };

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: None,
            mode: Mode::Normal,
            with_details: false,
        };

        #[allow(unused_variables)]
        let tokenizer = Tokenizer::new(config).unwrap();

        // tokenize the text
        let tokens = tokenizer.tokenize("한국어의형태해석을실시할수있습니다.")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

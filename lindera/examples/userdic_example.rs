#[cfg(feature = "ipadic")]
use std::path::PathBuf;

#[cfg(feature = "ipadic")]
use lindera::{
    mode::Mode,
    tokenizer::{DictionaryConfig, Tokenizer, TokenizerConfig, UserDictionaryConfig},
};
use lindera::{DictionaryKind, LinderaResult};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: PathBuf::from("./resources/ipadic_simple_userdic.csv"),
        });

        // create tokenizer
        let config = TokenizerConfig {
            dictionary,
            user_dictionary: user_dictionary,
            mode: Mode::Normal,
        };
        let tokenizer = Tokenizer::with_config(config)?;

        // tokenize the text
        let tokens =
            tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

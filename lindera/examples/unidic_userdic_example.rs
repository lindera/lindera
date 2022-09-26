#[cfg(feature = "unidic")]
use std::path::PathBuf;

use lindera::LinderaResult;
#[cfg(feature = "unidic")]
use lindera::{
    mode::Mode,
    tokenizer::{DictionaryConfig, Tokenizer, TokenizerConfig, UserDictionaryConfig},
    DictionaryKind,
};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "unidic")]
    {
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::UniDic,
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::UniDic,
            path: PathBuf::from("./resources/simple_userdic.csv"),
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

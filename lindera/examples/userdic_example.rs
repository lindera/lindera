#[cfg(feature = "ipadic")]
use std::path::PathBuf;

use lindera::LinderaResult;
#[cfg(feature = "ipadic")]
use lindera::{
    mode::Mode,
    tokenizer::{
        DictionaryConfig, DictionaryKind, DictionarySourceType, Tokenizer, TokenizerConfig,
        UserDictionaryConfig,
    },
};

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        let dictionary = DictionaryConfig {
            kind: DictionaryKind::IPADIC,
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: DictionaryKind::IPADIC,
            source_type: DictionarySourceType::Csv,
            path: PathBuf::from("./resources/userdic.csv"),
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

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
            kind: Some(DictionaryKind::UniDic),
            path: None,
        };

        let user_dictionary = Some(UserDictionaryConfig {
            kind: Some(DictionaryKind::UniDic),
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("unidic_simple_userdic.csv"),
        });

        let config = TokenizerConfig {
            dictionary,
            user_dictionary: user_dictionary,
            mode: Mode::Normal,
            with_details: false,
        };

        #[allow(unused_variables)]
        let tokenizer = Tokenizer::from_config(config).unwrap();

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

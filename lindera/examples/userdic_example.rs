#[cfg(feature = "ipadic")]
use std::path::PathBuf;

#[cfg(feature = "ipadic")]
use lindera::mode::Mode;
#[cfg(feature = "ipadic")]
use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        // create tokenizer
        let config = TokenizerConfig {
            user_dict_path: Some(PathBuf::from("./resources/userdic.csv")),
            mode: Mode::Normal,
            ..TokenizerConfig::default()
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

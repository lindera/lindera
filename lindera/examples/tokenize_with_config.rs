use lindera::core::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::tokenizer::{Tokenizer, TokenizerConfig};

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("lindera_ipadic_conf.json");

        let tokenizer_config = TokenizerConfig::from_file(&path).unwrap();

        let tokenizer = Tokenizer::from_config(&tokenizer_config).unwrap();

        let mut text =
            "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
        println!("text: {}", text);

        let tokens = tokenizer.tokenize(&mut text)?;

        for token in tokens {
            println!(
                "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
                token.text, token.byte_start, token.byte_end, token.details
            );
        }
    }

    Ok(())
}

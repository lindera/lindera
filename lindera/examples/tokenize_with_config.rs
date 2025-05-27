use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(feature = "ipadic")]
    {
        use std::path::PathBuf;

        use lindera::tokenizer::TokenizerBuilder;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../resources")
            .join("lindera.yml");

        let builder = TokenizerBuilder::from_file(&path)?;

        let tokenizer = builder.build()?;

        let text =
            "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
        println!("text: {}", text);

        let tokens = tokenizer.tokenize(&text)?;

        for token in tokens {
            println!(
                "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
                token.text, token.byte_start, token.byte_end, token.details
            );
        }
    }

    Ok(())
}

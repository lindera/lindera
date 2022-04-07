#[cfg(any(
    feature = "ipadic",
    feature = "unidic",
    feature = "ko-dic",
    feature = "cc-cedict"
))]
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    #[cfg(any(
        feature = "ipadic",
        feature = "unidic",
        feature = "ko-dic",
        feature = "cc-cedict"
    ))]
    {
        // create tokenizer
        let tokenizer = Tokenizer::new()?;

        // tokenize the text
        #[cfg(any(feature = "ipadic", feature = "unidic",))]
        let tokens = tokenizer.tokenize("日本語の形態素解析を行うことができます。")?;

        #[cfg(feature = "ko-dic")]
        let tokens = tokenizer.tokenize("한국어의형태해석을실시할수있습니다.")?;

        #[cfg(feature = "cc-cedict")]
        let tokens = tokenizer.tokenize("可以进行中文形态学分析。")?;

        // output the tokens
        for token in tokens {
            println!("{}", token.text);
        }
    }

    Ok(())
}

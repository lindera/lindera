use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // create tokenizer
    let tokenizer = Tokenizer::new()?;

    // tokenize the text
    let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ")?;

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}

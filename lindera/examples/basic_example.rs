use lindera::tokenizer::Tokenizer;

fn main() -> std::io::Result<()> {
    // create tokenizer
    let mut tokenizer = Tokenizer::new("normal", "");

    // tokenize the text
    let tokens = tokenizer.tokenize("関西国際空港限定トートバッグ");

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}

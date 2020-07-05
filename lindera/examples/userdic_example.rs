use lindera::tokenizer::Tokenizer;

fn main() -> std::io::Result<()> {
    // create tokenizer
    let mut tokenizer = Tokenizer::new_with_userdic("normal", "", "resources/userdic.csv");

    // tokenize the text
    let tokens = tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です");

    // output the tokens
    for token in tokens {
        println!("{}", token.text);
    }

    Ok(())
}

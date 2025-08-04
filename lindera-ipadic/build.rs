use std::error::Error;

#[cfg(feature = "ipadic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::assets::{FetchParams, fetch};
    // TODO: Phase 2 - Use local IpadicBuilder after refactoring build process
    use lindera_dictionary::dictionary_builder::ipadic::IpadicBuilder;

    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-2.7.0-20070801.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20070801",
        output_dir: "lindera-ipadic",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz"],
        md5_hash: "3311c7c71a869ca141e1b8bde0c8666c",
    };

    let builder = IpadicBuilder::default();

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

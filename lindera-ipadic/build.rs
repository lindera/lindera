use std::error::Error;

#[cfg(feature = "ipadic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "mecab-ipadic-2.7.0-20070801.tar.gz",
            input_dir: "mecab-ipadic-2.7.0-20070801",
            output_dir: "lindera-ipadic",
            dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
            download_urls: &[
                // "https://lindera.s3.ap-northeast-1.amazonaws.com/mecab-ipadic-2.7.0-20070801.tar.gz",
                "https://Lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz",
            ],
            md5_hash: "3311c7c71a869ca141e1b8bde0c8666c",
        },
        lindera_dictionary::dictionary_builder::ipadic::IpadicBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

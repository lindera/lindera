use std::error::Error;

#[cfg(feature = "ipadic")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "mecab-ipadic-2.7.0-20070801.tar.gz",
            input_dir: "mecab-ipadic-2.7.0-20070801",
            output_dir: "lindera-ipadic",
            download_urls: &[
                "https://lindera.s3.ap-northeast-1.amazonaws.com/mecab-ipadic-2.7.0-20070801.tar.gz",
                "https://pub-4a3be4c21bd441938069ef903b680944.r2.dev/mecab-ipadic-2.7.0-20070801.tar.gz"
            ],
            dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        },
        lindera_dictionary::dictionary_builder::ipadic::IpadicBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

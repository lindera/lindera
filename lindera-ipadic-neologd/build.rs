use std::error::Error;

#[cfg(feature = "ipadic-neologd")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
            input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
            output_dir: "lindera-ipadic-neologd",
            download_urls: &[
                ("https://lindera.s3.ap-northeast-1.amazonaws.com/mecab-ipadic-neologd-0.0.7-20200820.tar.gz", "https://lindera.s3.ap-northeast-1.amazonaws.com/mecab-ipadic-neologd-0.0.7-20200820.tar.gz.md5"),
                ("https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz", "https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz.md5"),
            ],
            dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        },
        lindera_dictionary::dictionary_builder::ipadic_neologd::IpadicNeologdBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "ipadic-neologd"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

use std::error::Error;

#[cfg(feature = "ipadic-neologd")]
fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
            input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
            output_dir: "lindera-ipadic-neologd",
            download_url:
                "https://dlwqk3ibdg1xh.cloudfront.net/mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
            dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        },
        lindera_dictionary::dictionary_builder::ipadic_neologd::IpadicNeologdBuilder::new(),
    )
}

#[cfg(not(feature = "ipadic-neologd"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

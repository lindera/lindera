use std::error::Error;

#[cfg(feature = "ipadic")]
fn main() -> Result<(), Box<dyn Error>> {
    lindera_assets::fetch(
        lindera_assets::FetchParams {
            file_name: "mecab-ipadic-2.7.0-20070801.tar.gz",
            input_dir: "mecab-ipadic-2.7.0-20070801",
            output_dir: "lindera-ipadic",
            download_url: "https://dlwqk3ibdg1xh.cloudfront.net/mecab-ipadic-2.7.0-20070801.tar.gz",
            dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        },
        lindera_ipadic_builder::ipadic_builder::IpadicBuilder::new(),
    )
}

#[cfg(not(feature = "ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

use std::error::Error;

#[cfg(feature = "ko-dic")]
fn main() -> Result<(), Box<dyn Error>> {
    lindera_core::assets::fetch(
        lindera_core::assets::FetchParams {
            file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
            input_dir: "mecab-ko-dic-2.1.1-20180720",
            output_dir: "lindera-ko-dic",
            download_url: "https://dlwqk3ibdg1xh.cloudfront.net/mecab-ko-dic-2.1.1-20180720.tar.gz",
            dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        },
        lindera_core::dictionary_builder::ko_dic::KoDicBuilder::new(),
    )
}

#[cfg(not(feature = "ko-dic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

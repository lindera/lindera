use std::error::Error;

#[cfg(feature = "ko-dic")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
            input_dir: "mecab-ko-dic-2.1.1-20180720",
            output_dir: "lindera-ko-dic",
            download_urls: &[
                "https://lindera.s3.ap-northeast-1.amazonaws.com/mecab-ko-dic-2.1.1-20180720.tar.gz",
                "https://pub-4a3be4c21bd441938069ef903b680944.r2.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"
            ],
            dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        },
        lindera_dictionary::dictionary_builder::ko_dic::KoDicBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "ko-dic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

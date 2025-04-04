use std::error::Error;

#[cfg(feature = "unidic")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "unidic-mecab-2.1.2.tar.gz",
            input_dir: "unidic-mecab-2.1.2",
            output_dir: "lindera-unidic",
            download_urls: &[
                "https://lindera.s3.ap-northeast-1.amazonaws.com/unidic-mecab-2.1.2.tar.gz",
                "https://pub-4a3be4c21bd441938069ef903b680944.r2.dev/unidic-mecab-2.1.2.tar.gz"
            ],
            dummy_input: "テスト,5131,5131,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
        },
        lindera_dictionary::dictionary_builder::unidic::UnidicBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "unidic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

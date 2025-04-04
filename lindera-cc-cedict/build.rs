use std::error::Error;

#[cfg(feature = "cc-cedict")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    lindera_dictionary::assets::fetch(
        lindera_dictionary::assets::FetchParams {
            file_name: "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
            input_dir: "CC-CEDICT-MeCab-0.1.0-20200409",
            output_dir: "lindera-cc-cedict",
            download_urls: &[
                "https://lindera.s3.ap-northeast-1.amazonaws.com/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
                "https://pub-4a3be4c21bd441938069ef903b680944.r2.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"
            ],
            dummy_input: "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
        },
        lindera_dictionary::dictionary_builder::cc_cedict::CcCedictBuilder::new(),
    )
    .await
}

#[cfg(not(feature = "cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

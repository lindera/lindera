use std::error::Error;

#[cfg(feature = "cc-cedict")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::assets::{FetchParams, fetch};
    use lindera_dictionary::dictionary_builder::cc_cedict::CcCedictBuilder;

    let fetch_params = FetchParams {
        file_name: "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
        input_dir: "CC-CEDICT-MeCab-0.1.0-20200409",
        output_dir: "lindera-cc-cedict",
        dummy_input: "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
        download_urls: &["https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"],
        md5_hash: "aba9748b70f37feede97b70c5d37f8a0",
    };

    let builder = CcCedictBuilder::default();

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

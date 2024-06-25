use std::error::Error;

#[cfg(feature = "cc-cedict")]
fn main() -> Result<(), Box<dyn Error>> {
    lindera_assets::fetch(lindera_assets::FetchParams {
        file_name: "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
        input_dir: "CC-CEDICT-MeCab-0.1.0-20200409",
        output_dir: "lindera-cc-cedict",
        download_url: "https://dlwqk3ibdg1xh.cloudfront.net/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
        dummy_input:
        "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
    },
    lindera_cc_cedict_builder::cc_cedict_builder::CcCedictBuilder::new())
}

#[cfg(not(feature = "cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

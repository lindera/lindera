use std::error::Error;

#[cfg(feature = "unidic")]
fn main() -> Result<(), Box<dyn Error>> {
    lindera_assets::fetch(lindera_assets::FetchParams {
        file_name: "unidic-mecab-2.1.2.tar.gz",
        input_dir: "unidic-mecab-2.1.2",
        output_dir: "lindera-unidic",
        download_url: "https://lindera.s3.ap-northeast-1.amazonaws.com/unidic-mecab-2.1.2.tar.gz",
        dummy_input: "テスト,5131,5131,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
    },
lindera_unidic_builder::unidic_builder::UnidicBuilder::new())
}

#[cfg(not(feature = "unidic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

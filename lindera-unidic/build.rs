use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "unidic-mecab-2.1.2.tar.gz",
        input_dir: "unidic-mecab-2.1.2",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-unidic",
        dummy_input: "テスト,0,0,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"],
        sha256: "24ac75fe4e44d973b55e65847c35fa4aec2db7a41a07b49ca125146db9fc1189",
    };

    build_embedded_dictionary(cfg!(feature = "embed-unidic"), fetch_params)
}

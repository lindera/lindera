use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "sudachidict-20260723.tar.gz",
        input_dir: "sudachidict-20260723",
        src_subdir: None,
        output_dir: "lindera-sudachidict",
        dummy_input: "テスト,0,0,767,テスト,名詞,普通名詞,一般,*,*,*,テスト,テスト,*,A,*,*,*,*\n",
        download_urls: &["https://lindera.dev/sudachidict-20260723.tar.gz"],
        sha256: "f27b7b7b3da722962edaa809b9175d3f00eb78e15158b41ceeec3812faac9e92",
    };

    build_embedded_dictionary(cfg!(feature = "embed-sudachidict"), fetch_params)
}

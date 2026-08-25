use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "sudachidict-20260723.tar.gz",
        input_dir: "sudachidict-20260723",
        src_subdir: None,
        output_dir: "lindera-sudachidict",
        dummy_input: "テスト,5131,5131,767,テスト,名詞,普通名詞,一般,*,*,*,テスト,テスト,*,A,*,*,*,*\n",
        download_urls: &["https://lindera.dev/sudachidict-20260723.tar.gz"],
        md5_hash: "f4cb9708a1d21f895ed49e8b66839283",
    };

    build_embedded_dictionary(cfg!(feature = "embed-sudachidict"), fetch_params)
}

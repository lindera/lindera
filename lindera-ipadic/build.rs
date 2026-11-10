use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-2.7.0-20250920.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20250920",
        src_subdir: None,
        output_dir: "lindera-ipadic",
        dummy_input: "テスト,0,0,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"],
        sha256: "a7ba9f645ffe7094e56ae1c4a81d100df8fbb1e28bbe1792622e9728e162db3d",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ipadic"), fetch_params)
}

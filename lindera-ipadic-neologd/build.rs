use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
        input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
        src_subdir: None,
        output_dir: "lindera-ipadic-neologd",
        dummy_input: "テスト,0,0,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"],
        sha256: "d55c02c204932858b179050554276a33364a9deffe15380ce7ac53edeb6a8ea2",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ipadic-neologd"), fetch_params)
}

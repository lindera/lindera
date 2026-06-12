use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-2.7.0-20250920.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20250920",
        src_subdir: None,
        output_dir: "lindera-ipadic",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"],
        md5_hash: "a95c409f12f1023fce8ef91f991ef042",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ipadic"), fetch_params).await
}

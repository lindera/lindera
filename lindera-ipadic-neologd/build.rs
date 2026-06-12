use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
        input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
        src_subdir: None,
        output_dir: "lindera-ipadic-neologd",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"],
        md5_hash: "3561f0e76980a842dc828b460a8cae96",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ipadic-neologd"), fetch_params).await
}

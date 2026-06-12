use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-jieba-0.1.1.tar.gz",
        input_dir: "mecab-jieba-0.1.1",
        src_subdir: Some("dict-src"),
        output_dir: "lindera-jieba",
        dummy_input: "1号店,1,1,1789,n,NUMERIC,*,*,*,*,3,1,店,low\n",
        download_urls: &["https://lindera.dev/mecab-jieba-0.1.1.tar.gz"],
        md5_hash: "749dc1ab25a035e141d014cd3c1cf8e9",
    };

    build_embedded_dictionary(cfg!(feature = "embed-jieba"), fetch_params).await
}

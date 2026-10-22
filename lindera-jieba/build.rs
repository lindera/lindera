use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-jieba-0.1.3.tar.gz",
        input_dir: "mecab-jieba-0.1.3",
        src_subdir: Some("dict-src"),
        output_dir: "lindera-jieba",
        dummy_input: "1号店,0,0,1789,n,NUMERIC,*,*,*,*,3,1,店,low\n",
        download_urls: &["https://lindera.dev/mecab-jieba-0.1.3.tar.gz"],
        md5_hash: "121fdf0caa1f3b2a304259cec8ab5dcf",
    };

    build_embedded_dictionary(cfg!(feature = "embed-jieba"), fetch_params)
}

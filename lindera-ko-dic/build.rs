use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        src_subdir: None,
        output_dir: "lindera-ko-dic",
        dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"],
        md5_hash: "b996764e91c96bc89dc32ea208514a96",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ko-dic"), fetch_params).await
}

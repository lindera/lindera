use std::error::Error;

use lindera_dictionary::assets::{FetchParams, build_embedded_dictionary};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "unidic-mecab-2.1.2.tar.gz",
        input_dir: "unidic-mecab-2.1.2",
        src_subdir: None,
        output_dir: "lindera-unidic",
        dummy_input: "テスト,5131,5131,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"],
        md5_hash: "f4502a563e1da44747f61dcd2b269e35",
    };

    build_embedded_dictionary(cfg!(feature = "embed-unidic"), fetch_params).await
}

use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-ko-dic",
        dummy_input: "테스트,0,0,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"],
        sha256: "702ced21c6167e9d9aebc674ab5ee54af58d4443975f2940d37d0567c020591a",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ko-dic"), fetch_params)
}

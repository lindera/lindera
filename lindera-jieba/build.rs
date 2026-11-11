use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-jieba-0.1.3.tar.gz",
        input_dir: "mecab-jieba-0.1.3",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: Some("dict-src"),
        output_dir: "lindera-jieba",
        dummy_input: "1号店,0,0,1789,n,NUMERIC,*,*,*,*,3,1,店,low\n",
        download_urls: &["https://lindera.dev/mecab-jieba-0.1.3.tar.gz"],
        sha256: "c5bc9490ba6c86f217f644424953257bd5661995c4da5a562698ee5364fca2e1",
    };

    build_embedded_dictionary(cfg!(feature = "embed-jieba"), fetch_params)
}

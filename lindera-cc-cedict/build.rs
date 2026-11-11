use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "CC-CEDICT-MeCab-0.1.0-20261021.tar.gz",
        input_dir: "CC-CEDICT-MeCab-0.1.0-20261021",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-cc-cedict",
        dummy_input: "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
        download_urls: &["https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20261021.tar.gz"],
        sha256: "82ba796fc97b66cae97957855a583fded076f9cdbe0ad0c997f8293fe1bd7a7f",
    };

    build_embedded_dictionary(cfg!(feature = "embed-cc-cedict"), fetch_params)
}

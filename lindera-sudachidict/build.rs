use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

fn main() -> Result<(), Box<dyn Error>> {
    // The mirror's 0.1.0-20260723: SudachiDict 20260723 with Sudachi's own
    // char.def (sudachi.rs v0.7.0), read in Sudachi's format (#1388).
    let fetch_params = FetchParams {
        file_name: "sudachidict-0.1.0-20260723.tar.gz",
        input_dir: "sudachidict-0.1.0-20260723",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-sudachidict",
        dummy_input: "テスト,0,0,767,テスト,名詞,普通名詞,一般,*,*,*,テスト,テスト,*,A,*,*,*,*\n",
        download_urls: &["https://lindera.dev/sudachidict-0.1.0-20260723.tar.gz"],
        sha256: "b878469728e99b8ece0fd348164473143084389c45ca5fe12bc7f73e4569ae42",
    };

    build_embedded_dictionary(cfg!(feature = "embed-sudachidict"), fetch_params)
}

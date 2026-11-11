use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

// The source is NINJAL's own package, downloaded from its official URL rather
// than hosted on lindera.dev (#216). It is the full package: the lighter one
// holds only MeCab's compiled files, not the lexicon and `matrix.def` the
// builder reads. Its files sit at the root of the zip, and only the ones the
// builder reads are extracted.
fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "unidic-cwj-202512_full.zip",
        input_dir: "unidic-cwj-202512",
        archive_layout: ArchiveLayout::Root,
        src_subdir: None,
        output_dir: "lindera-unidic-cwj",
        dummy_input: "テスト,0,0,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*,*,*,体,テスト,テスト,テスト,テスト,1,C1,*,0,0\n",
        download_urls: &[
            "https://clrd.ninjal.ac.jp/unidic_archive/2512/unidic-cwj-202512_full.zip",
        ],
        sha256: "ca02ad51c8ae238373dcb150eec014f1ad9f05f54c7416a16c2fe444fd4921b3",
    };

    build_embedded_dictionary(cfg!(feature = "embed-unidic-cwj"), fetch_params)
}

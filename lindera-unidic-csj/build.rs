use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

// The source is NINJAL's own package, downloaded from its official URL rather
// than hosted on lindera.dev, as for UniDic CWJ (#216, #1358). It is the full
// package: the lighter one holds only MeCab's compiled files, not the lexicon
// and `matrix.def` the builder reads. Its files sit at the root of the zip,
// and only the ones the builder reads are extracted.
fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "unidic-csj-202512_full.zip",
        input_dir: "unidic-csj-202512",
        archive_layout: ArchiveLayout::Root,
        src_subdir: None,
        output_dir: "lindera-unidic-csj",
        dummy_input: "テスト,0,0,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*,*,*,体,テスト,テスト,テスト,テスト,1,C1,*,0,0\n",
        download_urls: &[
            "https://clrd.ninjal.ac.jp/unidic_archive/2512/unidic-csj-202512_full.zip",
        ],
        sha256: "f737d26ce9daf96347e1ccaf34896a36651ffb7ceff91d4d8041591ff7a387e4",
    };

    build_embedded_dictionary(cfg!(feature = "embed-unidic-csj"), fetch_params)
}

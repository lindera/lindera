use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

// The source is NINJAL's own package of UniDic 2.1.2, downloaded from its
// official URL rather than hosted on lindera.dev (#1384), as for UniDic CWJ
// and UniDic CSJ. Its files sit under one top directory,
// `unidic-mecab-2.1.2_src`. lindera.dev's `unidic-mecab-2.1.2.tar.gz`, which
// v6 builds from, holds the same files repacked.
fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "unidic-mecab-2.1.2_src.zip",
        input_dir: "unidic-mecab-2.1.2_src",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-unidic",
        dummy_input: "テスト,0,0,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
        download_urls: &[
            "https://clrd.ninjal.ac.jp/unidic_archive/cwj/2.1.2/unidic-mecab-2.1.2_src.zip",
        ],
        sha256: "6cce98269214ce7de6159f61a25ffc5b436375c098cc86d6aa98c0605cbf90d4",
    };

    build_embedded_dictionary(cfg!(feature = "embed-unidic"), fetch_params)
}

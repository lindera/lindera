use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

// The source is mecab-ko-dic's official release, downloaded from the
// Downloads of the upstream repository on Bitbucket rather than hosted on
// lindera.dev (#1386); Bitbucket redirects the download once, to its storage.
// Its files sit under one top directory, `mecab-ko-dic-2.1.1-20180720`.
// lindera.dev's archive of the same name, which v6 builds from, holds the
// same files repacked, with a `.gitattributes` added.
fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: None,
        output_dir: "lindera-ko-dic",
        dummy_input: "테스트,0,0,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &[
            "https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/mecab-ko-dic-2.1.1-20180720.tar.gz",
        ],
        sha256: "fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330",
    };

    build_embedded_dictionary(cfg!(feature = "embed-ko-dic"), fetch_params)
}

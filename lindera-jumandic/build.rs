use std::error::Error;

use lindera_dictionary::assets::{ArchiveLayout, FetchParams, build_embedded_dictionary};

// The source is mecab-jumandic 7.0-20130310 as the repository of MeCab holds
// it, downloaded from GitHub rather than hosted on lindera.dev (#1466): the
// page of MeCab gives the archive of the dictionary through a file-sharing
// link, which a build cannot rely on. The archive is the one GitHub serves
// for a commit of taku910/mecab: the whole repository under one top
// directory, of which only `mecab-jumandic` is read.
fn main() -> Result<(), Box<dyn Error>> {
    let fetch_params = FetchParams {
        file_name: "mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz",
        input_dir: "mecab-61b90ba6e669dc2d7d533d4a80d206f3b31d52b1",
        archive_layout: ArchiveLayout::TopDirectory,
        src_subdir: Some("mecab-jumandic"),
        output_dir: "lindera-jumandic",
        dummy_input: "テスト,0,0,-1000,名詞,普通名詞,*,*,テスト,てすと,*\n",
        download_urls: &[
            "https://github.com/taku910/mecab/archive/61b90ba6e669dc2d7d533d4a80d206f3b31d52b1.tar.gz",
        ],
        sha256: "7ad44f987ae0b7fd345c72a4b67e14dbe0f4bd1669d247a8aaeb0f15218a3fd1",
    };

    build_embedded_dictionary(cfg!(feature = "embed-jumandic"), fetch_params)
}

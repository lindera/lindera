#[cfg(feature = "ipadic-neologd")]
use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(feature = "ipadic-neologd")]
use lindera::dictionary::{
    DictionaryKind, load_embedded_dictionary, load_user_dictionary_from_csv,
};
#[cfg(feature = "ipadic-neologd")]
use lindera::mode::Mode;
#[cfg(feature = "ipadic-neologd")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "ipadic-neologd")]
use lindera::tokenizer::Tokenizer;
#[cfg(feature = "ipadic-neologd")]
use std::fs::File;
#[cfg(feature = "ipadic-neologd")]
use std::io::{BufReader, Read};
#[cfg(feature = "ipadic-neologd")]
use std::path::PathBuf;

#[cfg(feature = "ipadic-neologd")]
fn bench_constructor_ipadic_neologd(c: &mut Criterion) {
    c.bench_function("bench-constructor-ipadic-neologd", |b| {
        b.iter(|| {
            let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "ipadic-neologd")]
fn bench_constructor_with_simple_userdic_ipadic_neologd(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-ipadic-neologd", |b| {
        b.iter(|| {
            let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("ipadic_simple_userdic.csv");

            let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
            let user_dictionary = load_user_dictionary_from_csv(
                DictionaryKind::IPADICNEologd,
                userdic_file.as_path(),
            )
            .unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "ipadic-neologd")]
fn bench_tokenize_ipadic_neologd(c: &mut Criterion) {
    let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-ipadic-neologd", |b| {
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

#[cfg(feature = "ipadic-neologd")]
fn bench_tokenize_with_simple_userdic_ipadic_neologd(c: &mut Criterion) {
    let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("ipadic_simple_userdic.csv");

    let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
    let user_dictionary =
        load_user_dictionary_from_csv(DictionaryKind::IPADICNEologd, userdic_file.as_path())
            .unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-ipadic-neologd", |b| {
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

#[cfg(feature = "ipadic-neologd")]
fn bench_tokenize_long_text_ipadic_neologd(c: &mut Criterion) {
    let mut long_text_file = BufReader::new(
        File::open(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("bocchan.txt"),
        )
        .unwrap(),
    );
    let mut long_text = String::new();
    let _size = long_text_file.read_to_string(&mut long_text).unwrap();

    let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-long-text-ipadic-neologd", |b| {
        b.iter(|| tokenizer.tokenize(long_text.as_str()));
    });
}

#[cfg(feature = "ipadic-neologd")]
fn bench_tokenize_details_long_text_ipadic_neologd(c: &mut Criterion) {
    let mut long_text_file = BufReader::new(
        File::open(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("bocchan.txt"),
        )
        .unwrap(),
    );
    let mut long_text = String::new();
    let _size = long_text_file.read_to_string(&mut long_text).unwrap();

    let dictionary = load_embedded_dictionary(DictionaryKind::IPADICNEologd).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-details-long-text-ipadic-neologd", |b| {
        b.iter(|| {
            let mut tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
            for token in tokens.iter_mut() {
                let _details = token.details();
            }
        });
    });
}

#[cfg(feature = "ipadic-neologd")]
criterion_group!(
    benches,
    bench_constructor_ipadic_neologd,
    bench_constructor_with_simple_userdic_ipadic_neologd,
    bench_tokenize_ipadic_neologd,
    bench_tokenize_with_simple_userdic_ipadic_neologd,
    bench_tokenize_long_text_ipadic_neologd,
    bench_tokenize_details_long_text_ipadic_neologd,
);

#[cfg(feature = "ipadic-neologd")]
criterion_main!(benches);

#[cfg(not(feature = "ipadic-neologd"))]
fn main() {
    println!("IPADIC-NEologd feature is not enabled");
}

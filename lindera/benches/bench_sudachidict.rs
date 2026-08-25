#[cfg(feature = "embed-sudachidict")]
use std::borrow::Cow;
#[cfg(feature = "embed-sudachidict")]
use std::fs::File;
#[cfg(feature = "embed-sudachidict")]
use std::io::{BufReader, Read};
#[cfg(feature = "embed-sudachidict")]
use std::path::PathBuf;

#[cfg(feature = "embed-sudachidict")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-sudachidict")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embed-sudachidict")]
use lindera::mode::Mode;
#[cfg(feature = "embed-sudachidict")]
use lindera::segmenter::Segmenter;

#[cfg(feature = "embed-sudachidict")]
fn bench_constructor_sudachidict(c: &mut Criterion) {
    c.bench_function("bench-constructor-sudachidict", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://sudachidict").unwrap();
            let _segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        })
    });
}

#[cfg(feature = "embed-sudachidict")]
fn bench_constructor_with_simple_userdic_sudachidict(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-sudachidict", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-sudachidict")
                .join("metadata.json");
            let metadata: Metadata = serde_json::from_reader(
                File::open(metadata_file)
                    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
                    .unwrap(),
            )
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
            .unwrap();

            let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("user_dict")
                .join("sudachidict_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://sudachidict").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let _segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
        })
    });
}

#[cfg(feature = "embed-sudachidict")]
fn bench_tokenize_sudachidict(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://sudachidict").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-sudachidict", |b| {
        b.iter(|| segmenter.segment(Cow::Borrowed("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。")))
    });
}

#[cfg(feature = "embed-sudachidict")]
fn bench_tokenize_with_simple_userdic_sudachidict(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-sudachidict")
        .join("metadata.json");
    let metadata: Metadata = serde_json::from_reader(
        File::open(metadata_file)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
            .unwrap(),
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
    .unwrap();

    let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("user_dict")
        .join("sudachidict_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://sudachidict").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

    c.bench_function("bench-tokenize-with-simple-userdic-sudachidict", |b| {
        b.iter(|| {
            segmenter.segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です",
            ))
        })
    });
}

#[cfg(feature = "embed-sudachidict")]
fn bench_tokenize_long_text_sudachidict(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://sudachidict").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-long-text-sudachidict", |b| {
        b.iter(|| segmenter.segment(Cow::Borrowed(long_text.as_str())));
    });
}

#[cfg(feature = "embed-sudachidict")]
fn bench_tokenize_details_long_text_sudachidict(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://sudachidict").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-details-long-text-sudachidict", |b| {
        b.iter(|| {
            let mut tokens = segmenter
                .segment(Cow::Borrowed(long_text.as_str()))
                .unwrap();
            for token in tokens.iter_mut() {
                let _details = token.details();
            }
        });
    });
}

#[cfg(feature = "embed-sudachidict")]
criterion_group!(
    benches,
    bench_constructor_sudachidict,
    bench_constructor_with_simple_userdic_sudachidict,
    bench_tokenize_sudachidict,
    bench_tokenize_with_simple_userdic_sudachidict,
    bench_tokenize_long_text_sudachidict,
    bench_tokenize_details_long_text_sudachidict,
);

#[cfg(feature = "embed-sudachidict")]
criterion_main!(benches);

#[cfg(not(feature = "embed-sudachidict"))]
fn main() {
    println!("Embedded SudachiDict feature is not enabled");
}

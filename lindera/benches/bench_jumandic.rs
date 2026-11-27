#[cfg(feature = "embed-jumandic")]
use std::borrow::Cow;
#[cfg(feature = "embed-jumandic")]
use std::fs::File;
#[cfg(feature = "embed-jumandic")]
use std::io::{BufReader, Read};
#[cfg(feature = "embed-jumandic")]
use std::path::PathBuf;

#[cfg(feature = "embed-jumandic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-jumandic")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embed-jumandic")]
use lindera::mode::Mode;
#[cfg(feature = "embed-jumandic")]
use lindera::segmenter::Segmenter;

#[cfg(feature = "embed-jumandic")]
fn bench_constructor_jumandic(c: &mut Criterion) {
    c.bench_function("bench-constructor-jumandic", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://jumandic").unwrap();
            let _segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        })
    });
}

#[cfg(feature = "embed-jumandic")]
fn bench_constructor_with_simple_userdic_jumandic(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-jumandic", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-jumandic")
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
                .join("jumandic_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://jumandic").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let _segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
        })
    });
}

#[cfg(feature = "embed-jumandic")]
fn bench_tokenize_jumandic(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://jumandic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-jumandic", |b| {
        b.iter(|| segmenter.segment(Cow::Borrowed("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。")))
    });
}

#[cfg(feature = "embed-jumandic")]
fn bench_tokenize_with_simple_userdic_jumandic(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-jumandic")
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
        .join("jumandic_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://jumandic").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

    c.bench_function("bench-tokenize-with-simple-userdic-jumandic", |b| {
        b.iter(|| {
            segmenter.segment(Cow::Borrowed(
                "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です",
            ))
        })
    });
}

#[cfg(feature = "embed-jumandic")]
fn bench_tokenize_long_text_jumandic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://jumandic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-long-text-jumandic", |b| {
        b.iter(|| segmenter.segment(Cow::Borrowed(long_text.as_str())));
    });
}

#[cfg(feature = "embed-jumandic")]
fn bench_tokenize_details_long_text_jumandic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://jumandic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    c.bench_function("bench-tokenize-details-long-text-jumandic", |b| {
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

#[cfg(feature = "embed-jumandic")]
criterion_group!(
    benches,
    bench_constructor_jumandic,
    bench_constructor_with_simple_userdic_jumandic,
    bench_tokenize_jumandic,
    bench_tokenize_with_simple_userdic_jumandic,
    bench_tokenize_long_text_jumandic,
    bench_tokenize_details_long_text_jumandic,
);

#[cfg(feature = "embed-jumandic")]
criterion_main!(benches);

#[cfg(not(feature = "embed-jumandic"))]
fn main() {
    println!("Embedded JUMAN dictionary feature is not enabled");
}

#[cfg(feature = "embedded-unidic")]
use std::fs::File;
#[cfg(feature = "embedded-unidic")]
use std::io::{BufReader, Read};
#[cfg(feature = "embedded-unidic")]
use std::path::PathBuf;

#[cfg(feature = "embedded-unidic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embedded-unidic")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embedded-unidic")]
use lindera::mode::Mode;
#[cfg(feature = "embedded-unidic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embedded-unidic")]
use lindera::tokenizer::Tokenizer;

#[cfg(feature = "embedded-unidic")]
fn bench_constructor_unidic(c: &mut Criterion) {
    c.bench_function("bench-constructor-unidic", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://unidic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embedded-unidic")]
fn bench_constructor_with_simple_userdic_unidic(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-unidic", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-unidic")
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
                .join("unidic_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://unidic").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embedded-unidic")]
fn bench_tokenize_unidic(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://unidic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-unidic", |b| {
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

#[cfg(feature = "embedded-unidic")]
fn bench_tokenize_with_simple_userdic_unidic(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-unidic")
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
        .join("unidic_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://unidic").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-unidic", |b| {
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

#[cfg(feature = "embedded-unidic")]
fn bench_tokenize_long_text_unidic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://unidic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-long-text-unidic", |b| {
        b.iter(|| tokenizer.tokenize(long_text.as_str()));
    });
}

#[cfg(feature = "embedded-unidic")]
fn bench_tokenize_details_long_text_unidic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://unidic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-details-long-text-unidic", |b| {
        b.iter(|| {
            let mut tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
            for token in tokens.iter_mut() {
                let _details = token.details();
            }
        });
    });
}

#[cfg(feature = "embedded-unidic")]
criterion_group!(
    benches,
    bench_constructor_unidic,
    bench_constructor_with_simple_userdic_unidic,
    bench_tokenize_unidic,
    bench_tokenize_with_simple_userdic_unidic,
    bench_tokenize_long_text_unidic,
    bench_tokenize_details_long_text_unidic,
);

#[cfg(feature = "embedded-unidic")]
criterion_main!(benches);

#[cfg(not(feature = "embedded-unidic"))]
fn main() {
    println!("Embedded UniDic feature is not enabled");
}

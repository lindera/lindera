#[cfg(feature = "embedded-ipadic")]
use std::fs::File;
#[cfg(feature = "embedded-ipadic")]
use std::io::{BufReader, Read};
#[cfg(feature = "embedded-ipadic")]
use std::path::PathBuf;

#[cfg(feature = "embedded-ipadic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embedded-ipadic")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embedded-ipadic")]
use lindera::mode::Mode;
#[cfg(feature = "embedded-ipadic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embedded-ipadic")]
use lindera::tokenizer::Tokenizer;

#[cfg(feature = "embedded-ipadic")]
fn bench_constructor_ipadic(c: &mut Criterion) {
    c.bench_function("bench-constructor-ipadic", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://ipadic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embedded-ipadic")]
fn bench_constructor_with_simple_userdic_ipadic(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-ipadic", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-ipadic")
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
                .join("ipadic_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://ipadic").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embedded-ipadic")]
fn bench_tokenize_ipadic(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-ipadic", |b| {
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

#[cfg(feature = "embedded-ipadic")]
fn bench_tokenize_with_simple_userdic_ipadic(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ipadic")
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
        .join("ipadic_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-ipadic", |b| {
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

#[cfg(feature = "embedded-ipadic")]
fn bench_tokenize_long_text_ipadic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-long-text-ipadic", |b| {
        b.iter(|| tokenizer.tokenize(long_text.as_str()));
    });
}

#[cfg(feature = "embedded-ipadic")]
fn bench_tokenize_details_long_text_ipadic(c: &mut Criterion) {
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

    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-details-long-text-ipadic", |b| {
        b.iter(|| {
            let mut tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
            for token in tokens.iter_mut() {
                let _details = token.details();
            }
        });
    });
}

#[cfg(feature = "embedded-ipadic")]
criterion_group!(
    benches,
    bench_constructor_ipadic,
    bench_constructor_with_simple_userdic_ipadic,
    bench_tokenize_ipadic,
    bench_tokenize_with_simple_userdic_ipadic,
    bench_tokenize_long_text_ipadic,
    bench_tokenize_details_long_text_ipadic,
);

#[cfg(feature = "embedded-ipadic")]
criterion_main!(benches);

#[cfg(not(feature = "embedded-ipadic"))]
fn main() {
    println!("Embedded IPADIC feature is not enabled");
}

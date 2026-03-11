#[cfg(feature = "embed-jieba")]
use std::path::PathBuf;

#[cfg(feature = "embed-jieba")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-jieba")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embed-jieba")]
use lindera::mode::Mode;
#[cfg(feature = "embed-jieba")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embed-jieba")]
use lindera::tokenizer::Tokenizer;

#[cfg(feature = "embed-jieba")]
fn bench_constructor_jieba(c: &mut Criterion) {
    c.bench_function("bench-constructor-jieba", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://jieba").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embed-jieba")]
fn bench_constructor_with_simple_userdic_jieba(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-jieba", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-jieba")
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
                .join("jieba_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://jieba").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embed-jieba")]
fn bench_tokenize_jieba(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://jieba").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-jieba", |b| {
        b.iter(|| tokenizer.tokenize("搜索引擎（英語：search engine）是一种信息检索系统，旨在协助搜索存储在计算机系统中的信息。"))
    });
}

#[cfg(feature = "embed-jieba")]
fn bench_tokenize_with_simple_userdic_jieba(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-jieba")
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
        .join("jieba_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://jieba").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-jieba", |b| {
        b.iter(|| tokenizer.tokenize("羽田机场限定托特包。"))
    });
}

#[cfg(feature = "embed-jieba")]
criterion_group!(
    benches,
    bench_constructor_jieba,
    bench_constructor_with_simple_userdic_jieba,
    bench_tokenize_jieba,
    bench_tokenize_with_simple_userdic_jieba,
);

#[cfg(feature = "embed-jieba")]
criterion_main!(benches);

#[cfg(not(feature = "embed-jieba"))]
fn main() {
    println!("Embedded Jieba feature is not enabled");
}

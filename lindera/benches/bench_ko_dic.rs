#[cfg(feature = "embed-ko-dic")]
use std::path::PathBuf;

#[cfg(feature = "embed-ko-dic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-ko-dic")]
use lindera::dictionary::{load_dictionary, load_user_dictionary};
#[cfg(feature = "embed-ko-dic")]
use lindera::mode::Mode;
#[cfg(feature = "embed-ko-dic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embed-ko-dic")]
use lindera::tokenizer::Tokenizer;

#[cfg(feature = "embed-ko-dic")]
fn bench_constructor_ko_dic(c: &mut Criterion) {
    c.bench_function("bench-constructor-ko-dic", |b| {
        b.iter(|| {
            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embed-ko-dic")]
fn bench_constructor_with_simple_userdic_ko_dic(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-ko-dic", |b| {
        b.iter(|| {
            use std::fs::File;

            use lindera::dictionary::Metadata;
            use lindera::error::LinderaErrorKind;

            let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../lindera-ko-dic")
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
                .join("ko-dic_simple_userdic.csv");

            let dictionary = load_dictionary("embedded://ko-dic").unwrap();
            let user_dictionary =
                load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "embed-ko-dic")]
fn bench_tokenize_ko_dic(c: &mut Criterion) {
    let dictionary = load_dictionary("embedded://ko-dic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-ko-dic", |b| {
        b.iter(|| tokenizer.tokenize("검색엔진(search engine)은컴퓨터시스템에저장된정보를찾아주거나웹검색(web search query)을도와주도록설계된정보검색시스템또는컴퓨터프로그램이다. 이러한검색결과는목록으로표시되는것이보통이다."))
    });
}

#[cfg(feature = "embed-ko-dic")]
fn bench_tokenize_with_simple_userdic_ko_dic(c: &mut Criterion) {
    use std::fs::File;

    use lindera::dictionary::Metadata;
    use lindera::error::LinderaErrorKind;

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ko-dic")
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
        .join("ko-dic_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://ko-dic").unwrap();
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-ko-dic", |b| {
        b.iter(|| tokenizer.tokenize("하네다공항한정토트백."))
    });
}

#[cfg(feature = "embed-ko-dic")]
criterion_group!(
    benches,
    bench_constructor_ko_dic,
    bench_constructor_with_simple_userdic_ko_dic,
    bench_tokenize_ko_dic,
    bench_tokenize_with_simple_userdic_ko_dic,
);

#[cfg(feature = "embed-ko-dic")]
criterion_main!(benches);

#[cfg(not(feature = "embed-ko-dic"))]
fn main() {
    println!("Embedded KO-DIC feature is not enabled");
}

#[cfg(feature = "ko-dic")]
use criterion::{Criterion, criterion_group, criterion_main};
#[cfg(feature = "ko-dic")]
use lindera::dictionary::{
    DictionaryKind, load_embedded_dictionary, load_user_dictionary_from_csv,
};
#[cfg(feature = "ko-dic")]
use lindera::mode::Mode;
#[cfg(feature = "ko-dic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "ko-dic")]
use lindera::tokenizer::Tokenizer;
#[cfg(feature = "ko-dic")]
use std::path::PathBuf;

#[cfg(feature = "ko-dic")]
fn bench_constructor_ko_dic(c: &mut Criterion) {
    c.bench_function("bench-constructor-ko-dic", |b| {
        b.iter(|| {
            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "ko-dic")]
fn bench_constructor_with_simple_userdic_ko_dic(c: &mut Criterion) {
    c.bench_function("bench-constructor-simple-userdic-ko-dic", |b| {
        b.iter(|| {
            let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("ko-dic_simple_userdic.csv");

            let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
            let user_dictionary =
                load_user_dictionary_from_csv(DictionaryKind::KoDic, userdic_file.as_path())
                    .unwrap();
            let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
            let _tokenizer = Tokenizer::new(segmenter);
        })
    });
}

#[cfg(feature = "ko-dic")]
fn bench_tokenize_ko_dic(c: &mut Criterion) {
    let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-ko-dic", |b| {
        b.iter(|| tokenizer.tokenize("검색엔진(search engine)은컴퓨터시스템에저장된정보를찾아주거나웹검색(web search query)을도와주도록설계된정보검색시스템또는컴퓨터프로그램이다. 이러한검색결과는목록으로표시되는것이보통이다."))
    });
}

#[cfg(feature = "ko-dic")]
fn bench_tokenize_with_simple_userdic_ko_dic(c: &mut Criterion) {
    let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("ko-dic_simple_userdic.csv");

    let dictionary = load_embedded_dictionary(DictionaryKind::KoDic).unwrap();
    let user_dictionary =
        load_user_dictionary_from_csv(DictionaryKind::KoDic, userdic_file.as_path()).unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));
    let tokenizer = Tokenizer::new(segmenter);

    c.bench_function("bench-tokenize-with-simple-userdic-ko-dic", |b| {
        b.iter(|| tokenizer.tokenize("하네다공항한정토트백."))
    });
}

#[cfg(feature = "ko-dic")]
criterion_group!(
    benches,
    bench_constructor_ko_dic,
    bench_constructor_with_simple_userdic_ko_dic,
    bench_tokenize_ko_dic,
    bench_tokenize_with_simple_userdic_ko_dic,
);

#[cfg(feature = "ko-dic")]
criterion_main!(benches);

#[cfg(not(feature = "ko-dic"))]
fn main() {
    println!("KO-DIC feature is not enabled");
}

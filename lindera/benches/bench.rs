use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use lindera::mode::Mode;
use lindera::tokenizer::{DictionaryConfig, Tokenizer, TokenizerConfig, UserDictionaryConfig};
use lindera::DictionaryKind;

fn bench_constructor(c: &mut Criterion) {
    c.bench_function("bench-constructor", |b| {
        b.iter(|| Tokenizer::new().unwrap())
    });
}

fn bench_constructor_with_custom_dict(c: &mut Criterion) {
    c.bench_function("bench-constructor-custom-dict", |b| {
        b.iter(|| {
            let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../resources")
                .join("simple_userdic.csv");

            let dictionary = DictionaryConfig {
                kind: DictionaryKind::IPADIC,
                path: None,
            };

            let user_dictionary = Some(UserDictionaryConfig {
                kind: DictionaryKind::IPADIC,
                path: userdic_file,
            });

            let config = TokenizerConfig {
                dictionary,
                user_dictionary,
                mode: Mode::Normal,
            };
            Tokenizer::with_config(config).unwrap()
        })
    });
}

fn bench_tokenize(c: &mut Criterion) {
    let tokenizer = Tokenizer::new().unwrap();
    c.bench_function("bench-tokenize-wiki", |b| {
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

fn bench_tokenize_with_custom_dict(c: &mut Criterion) {
    let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("simple_userdic.csv");

    let dictionary = DictionaryConfig {
        kind: DictionaryKind::IPADIC,
        path: None,
    };

    let user_dictionary = Some(UserDictionaryConfig {
        kind: DictionaryKind::IPADIC,
        path: userdic_file,
    });

    let config = TokenizerConfig {
        dictionary,
        user_dictionary,
        mode: Mode::Normal,
    };

    let tokenizer = Tokenizer::with_config(config).unwrap();
    c.bench_function("bench-tokenize-custom-dict", |b| {
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

fn bench_tokenize_long_text(c: &mut Criterion) {
    let mut long_text_file = BufReader::new(File::open("../resources/bocchan.txt").unwrap());
    let mut long_text = String::new();
    let _size = long_text_file.read_to_string(&mut long_text).unwrap();
    let tokenizer = Tokenizer::new().unwrap();
    // Using benchmark_group for changing sample_size
    let mut group = c.benchmark_group("long-text");
    group.sample_size(20);
    group.bench_function("bench-tokenize-long-text", |b| {
        b.iter(|| tokenizer.tokenize(long_text.as_str()));
    });
    group.finish();
}

fn bench_tokenize_details_long_text(c: &mut Criterion) {
    let mut long_text_file = BufReader::new(File::open("../resources/bocchan.txt").unwrap());
    let mut long_text = String::new();
    let _size = long_text_file.read_to_string(&mut long_text).unwrap();
    let tokenizer = Tokenizer::new().unwrap();
    // Using benchmark_group for changing sample_size
    let mut group = c.benchmark_group("long-text");
    group.sample_size(20);
    group.bench_function("bench-tokenize-details-long-text", |b| {
        b.iter(|| {
            let tokens = tokenizer.tokenize(long_text.as_str()).unwrap();
            for token in tokens {
                tokenizer.word_detail(token.word_id).unwrap();
            }
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_constructor,
    bench_constructor_with_custom_dict,
    bench_tokenize,
    bench_tokenize_with_custom_dict,
    bench_tokenize_long_text,
    bench_tokenize_details_long_text,
);
criterion_main!(benches);

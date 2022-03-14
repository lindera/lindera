use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use lindera::mode::Mode;
use lindera::tokenizer::{Tokenizer, TokenizerConfig, UserDictionaryType};

fn bench_constructor(c: &mut Criterion) {
    c.bench_function("bench-constructor", |b| {
        b.iter(|| Tokenizer::new().unwrap())
    });
}

fn bench_constructor_with_custom_dict(c: &mut Criterion) {
    c.bench_function("bench-constructor-custom-dict", |b| {
        b.iter(|| {
            let config = TokenizerConfig {
                user_dict_path: Some(PathBuf::from("../resources/userdic.csv")),
                user_dict_type: UserDictionaryType::Csv,
                mode: Mode::Normal,
                ..TokenizerConfig::default()
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
    let config = TokenizerConfig {
        user_dict_path: Some(PathBuf::from("../resources/userdic.csv")),
        user_dict_type: UserDictionaryType::Csv,
        mode: Mode::Normal,
        ..TokenizerConfig::default()
    };
    let tokenizer = Tokenizer::with_config(config).unwrap();
    c.bench_function("bench-tokenize-custom-dict", |b| {
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

fn bench_tokenize_long_text(c: &mut Criterion) {
    let mut large_file = BufReader::new(File::open("../resources/bocchan.txt").unwrap());
    let mut large_text = String::new();
    let _size = large_file.read_to_string(&mut large_text).unwrap();
    let tokenizer = Tokenizer::new().unwrap();
    // Using benchmark_group for changing sample_size
    let mut group = c.benchmark_group("Long text");
    group.sample_size(20);
    group.bench_function("bench-tokenize-long-text", |b| {
        b.iter(|| tokenizer.tokenize(large_text.as_str()));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_constructor,
    bench_constructor_with_custom_dict,
    bench_tokenize,
    bench_tokenize_with_custom_dict,
    bench_tokenize_long_text
);
criterion_main!(benches);

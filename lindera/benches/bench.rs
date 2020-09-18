use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use lindera::tokenizer::Tokenizer;
use std::fs::File;
use std::io::{BufReader, Read};

fn bench_tokenize(c: &mut Criterion) {
    c.bench_function("bench-wiki", |b| {
        let mut tokenizer = Tokenizer::new("normal", "");
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

fn bench_tokenize_with_custom_dict(c: &mut Criterion) {
    c.bench_function("bench-custom-dict", |b| {
        let mut tokenizer = Tokenizer::new_with_userdic("normal", "", "resources/userdic.csv");
        b.iter(|| tokenizer.tokenize("東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"))
    });
}

fn bench_long_text(c: &mut Criterion) {
    let mut large_file = BufReader::new(File::open("resources/bocchan.txt").unwrap());
    let mut large_text = String::new();
    let _size = large_file.read_to_string(&mut large_text).unwrap();
    // Using benchmark_group for changing sample_size
    let mut group = c.benchmark_group("Long text");
    group.sample_size(20);
    group.bench_function("bench-long-text", |b| {
        let mut tokenizer = Tokenizer::new("normal", "");
        b.iter(|| tokenizer.tokenize(large_text.as_str()));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_tokenize,
    bench_tokenize_with_custom_dict,
    bench_long_text
);
criterion_main!(benches);

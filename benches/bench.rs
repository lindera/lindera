use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use lindera::core::tokenizer::Tokenizer;

fn bench_tokenize(c: &mut Criterion) {
    c.bench_function("bench-wiki", |b| {
        let mut tokenizer = Tokenizer::normal();
        b.iter(|| tokenizer.tokenize("検索エンジン（けんさくエンジン、英語: search engine）は、狭義にはインターネットに存在する情報（ウェブページ、ウェブサイト、画像ファイル、ネットニュースなど）を検索する機能およびそのプログラム。"))
    });
}

criterion_group!(benches, bench_tokenize);
criterion_main!(benches);

//! Binding-layer benchmarks: full `TokenView` materialization (details
//! loaded and copied per token) vs the surface-only `tokenize_surfaces`
//! fast path. This is the primary evidence for the C3 claim of #881.

#[cfg(feature = "embed-ipadic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-ipadic")]
use lindera_binding_core::tokenizer::CoreTokenizer;

#[cfg(feature = "embed-ipadic")]
const SHORT_TEXT: &str = "すもももももももものうち";

#[cfg(feature = "embed-ipadic")]
fn build_tokenizer() -> CoreTokenizer {
    let dictionary = lindera::dictionary::load_dictionary("embedded://ipadic").unwrap();
    CoreTokenizer::from_segmenter("normal", dictionary, None).unwrap()
}

#[cfg(feature = "embed-ipadic")]
fn bench_binding_tokenize_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer();
    c.bench_function("bench-binding-tokenize-ipadic", |b| {
        b.iter(|| tokenizer.tokenize(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_binding_tokenize_surfaces_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer();
    c.bench_function("bench-binding-tokenize-surfaces-ipadic", |b| {
        b.iter(|| tokenizer.tokenize_surfaces(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
criterion_group!(
    benches,
    bench_binding_tokenize_ipadic,
    bench_binding_tokenize_surfaces_ipadic,
);

#[cfg(feature = "embed-ipadic")]
criterion_main!(benches);

#[cfg(not(feature = "embed-ipadic"))]
fn main() {
    eprintln!("bench_binding_core_ipadic requires --features embed-ipadic");
}

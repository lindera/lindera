//! Analysis-chain benchmarks: fresh `Tokenizer::tokenize` (a new lattice
//! per call) vs the reusable `AnalysisWorker`, with and without filters.
//! All benches return the token count so the worker variants (whose tokens
//! borrow the worker and cannot leave the closure) stay comparable with
//! the fresh variants.

#[cfg(feature = "embed-ipadic")]
use std::collections::HashMap;

#[cfg(feature = "embed-ipadic")]
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-ipadic")]
use lindera::dictionary::load_dictionary;
#[cfg(feature = "embed-ipadic")]
use lindera::mode::Mode;
#[cfg(feature = "embed-ipadic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::character_filter::BoxCharacterFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::character_filter::mapping::MappingCharacterFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::token_filter::BoxTokenFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::token_filter::lowercase::LowercaseTokenFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::tokenizer::Tokenizer;

#[cfg(feature = "embed-ipadic")]
const SHORT_TEXT: &str = "すもももももももものうち";

#[cfg(feature = "embed-ipadic")]
fn build_tokenizer(with_filters: bool) -> Tokenizer {
    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let mut tokenizer = Tokenizer::new(segmenter);
    if with_filters {
        let mut mapping = HashMap::new();
        mapping.insert("リンデラ".to_string(), "Lindera".to_string());
        let filter = MappingCharacterFilter::new(mapping).unwrap();
        tokenizer
            .character_filters
            .push(BoxCharacterFilter::from(filter));
        tokenizer
            .token_filters
            .push(BoxTokenFilter::from(LowercaseTokenFilter::new()));
    }
    tokenizer
}

#[cfg(feature = "embed-ipadic")]
fn bench_analysis_tokenize_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer(false);
    c.bench_function("bench-analysis-tokenize-ipadic", |b| {
        b.iter(|| tokenizer.tokenize(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_analysis_worker_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer(false);
    c.bench_function("bench-analysis-worker-ipadic", |b| {
        let mut worker = tokenizer.new_worker();
        b.iter(|| worker.tokenize(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_analysis_tokenize_filters_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer(true);
    c.bench_function("bench-analysis-tokenize-filters-ipadic", |b| {
        b.iter(|| tokenizer.tokenize(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_analysis_worker_filters_ipadic(c: &mut Criterion) {
    let tokenizer = build_tokenizer(true);
    c.bench_function("bench-analysis-worker-filters-ipadic", |b| {
        let mut worker = tokenizer.new_worker();
        b.iter(|| worker.tokenize(SHORT_TEXT).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
criterion_group!(
    benches,
    bench_analysis_tokenize_ipadic,
    bench_analysis_worker_ipadic,
    bench_analysis_tokenize_filters_ipadic,
    bench_analysis_worker_filters_ipadic,
);

#[cfg(feature = "embed-ipadic")]
criterion_main!(benches);

#[cfg(not(feature = "embed-ipadic"))]
fn main() {
    eprintln!("bench_analysis_ipadic requires --features embed-ipadic");
}

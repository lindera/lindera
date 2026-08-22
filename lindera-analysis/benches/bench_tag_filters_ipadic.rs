//! Tag token filter benchmarks (#967).
//!
//! The four tag filters (`japanese_stop_tags`, `japanese_keep_tags`,
//! `korean_stop_tags`, `korean_keep_tags`) share one helper,
//! `apply_tag_filter`, whose per-token cost is what these benches target.
//! Before #967 that helper allocated a comparison-key `String` per token, a
//! `Vec<&str>` per token for the Japanese key, and one replacement token
//! vector per call; it now reuses a single key buffer and filters in place.
//!
//! Two shapes are measured:
//!
//! - `apply-*`: [`TokenFilter::apply`] alone, over a token vector built (and
//!   its details materialized) outside the timed region. This isolates the
//!   filter's own cost, which is what changed.
//! - `pipeline-*`: the full `Tokenizer::tokenize` with the filter configured,
//!   so the filter's share of a realistic analysis chain stays visible.
//!
//! The corpus is `resources/bocchan.txt`, which yields a large token count --
//! the per-token cost is invisible on the 12-character text the other
//! analysis benches use.

#[cfg(feature = "embed-ipadic")]
use std::borrow::Cow;
#[cfg(feature = "embed-ipadic")]
use std::fs;
#[cfg(feature = "embed-ipadic")]
use std::path::PathBuf;

#[cfg(feature = "embed-ipadic")]
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

#[cfg(feature = "embed-ipadic")]
use lindera::dictionary::load_dictionary;
#[cfg(feature = "embed-ipadic")]
use lindera::mode::Mode;
#[cfg(feature = "embed-ipadic")]
use lindera::segmenter::Segmenter;
#[cfg(feature = "embed-ipadic")]
use lindera::token::Token;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::token_filter::japanese_keep_tags::JapaneseKeepTagsTokenFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::token_filter::{BoxTokenFilter, TokenFilter};
#[cfg(feature = "embed-ipadic")]
use lindera_analysis::tokenizer::Tokenizer;

/// The 25 stop tags configured in `resources/config/lindera.yml`, i.e. what
/// the shipped default pipeline actually runs.
#[cfg(feature = "embed-ipadic")]
const DEFAULT_STOP_TAGS: &[&str] = &[
    "接続詞",
    "助詞",
    "助詞,格助詞",
    "助詞,格助詞,一般",
    "助詞,格助詞,引用",
    "助詞,格助詞,連語",
    "助詞,係助詞",
    "助詞,副助詞",
    "助詞,間投助詞",
    "助詞,並立助詞",
    "助詞,終助詞",
    "助詞,副助詞／並立助詞／終助詞",
    "助詞,連体化",
    "助詞,副詞化",
    "助詞,特殊",
    "助動詞",
    "記号",
    "記号,一般",
    "記号,読点",
    "記号,句点",
    "記号,空白",
    "記号,括弧閉",
    "記号,括弧開",
    "記号,アルファベット",
    "その他,間投",
];

/// Keep-tag counterpart: the content-word tags a search pipeline typically
/// retains.
#[cfg(feature = "embed-ipadic")]
const KEEP_TAGS: &[&str] = &["名詞", "動詞", "形容詞", "副詞"];

#[cfg(feature = "embed-ipadic")]
fn corpus() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("bocchan.txt");
    fs::read_to_string(&path).unwrap()
}

#[cfg(feature = "embed-ipadic")]
fn segmenter() -> Segmenter {
    let dictionary = load_dictionary("embedded://ipadic").unwrap();
    Segmenter::new(Mode::Normal, dictionary, None)
}

#[cfg(feature = "embed-ipadic")]
fn tags_config(tags: &[&str]) -> serde_json::Value {
    serde_json::json!({ "tags": tags })
}

/// Segments `text` and materializes every token's details, so a caller
/// timing only `apply` does not measure the detail loading the first
/// accessor call would otherwise trigger inside the filter.
#[cfg(feature = "embed-ipadic")]
fn prepared_tokens<'a>(segmenter: &'a Segmenter, text: &'a str) -> Vec<Token<'a>> {
    let mut tokens = segmenter.segment(Cow::Borrowed(text)).unwrap();
    for token in tokens.iter_mut() {
        let _ = token.details_iter().count();
    }
    tokens
}

#[cfg(feature = "embed-ipadic")]
fn bench_apply_japanese_stop_tags(c: &mut Criterion) {
    let text = corpus();
    let segmenter = segmenter();
    let filter = JapaneseStopTagsTokenFilter::from_config(&tags_config(DEFAULT_STOP_TAGS)).unwrap();

    c.bench_function("bench-tag-filters-apply-japanese-stop-tags-ipadic", |b| {
        b.iter_batched(
            || prepared_tokens(&segmenter, &text),
            |mut tokens| {
                filter.apply(&mut tokens).unwrap();
                tokens.len()
            },
            BatchSize::LargeInput,
        )
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_apply_japanese_keep_tags(c: &mut Criterion) {
    let text = corpus();
    let segmenter = segmenter();
    let filter = JapaneseKeepTagsTokenFilter::from_config(&tags_config(KEEP_TAGS)).unwrap();

    c.bench_function("bench-tag-filters-apply-japanese-keep-tags-ipadic", |b| {
        b.iter_batched(
            || prepared_tokens(&segmenter, &text),
            |mut tokens| {
                filter.apply(&mut tokens).unwrap();
                tokens.len()
            },
            BatchSize::LargeInput,
        )
    });
}

/// The empty-tag-set fast path, which must stay free: it resolves the whole
/// call without ever building a key.
#[cfg(feature = "embed-ipadic")]
fn bench_apply_empty_tag_set(c: &mut Criterion) {
    let text = corpus();
    let segmenter = segmenter();
    let filter = JapaneseStopTagsTokenFilter::from_config(&tags_config(&[])).unwrap();

    c.bench_function("bench-tag-filters-apply-empty-tag-set-ipadic", |b| {
        b.iter_batched(
            || prepared_tokens(&segmenter, &text),
            |mut tokens| {
                filter.apply(&mut tokens).unwrap();
                tokens.len()
            },
            BatchSize::LargeInput,
        )
    });
}

#[cfg(feature = "embed-ipadic")]
fn bench_pipeline_with_stop_tags(c: &mut Criterion) {
    let text = corpus();
    let mut tokenizer = Tokenizer::new(segmenter());
    let filter = JapaneseStopTagsTokenFilter::from_config(&tags_config(DEFAULT_STOP_TAGS)).unwrap();
    tokenizer.token_filters.push(BoxTokenFilter::from(filter));

    c.bench_function("bench-tag-filters-pipeline-stop-tags-ipadic", |b| {
        let mut worker = tokenizer.new_worker();
        b.iter(|| worker.tokenize(&text).unwrap().len())
    });
}

/// Baseline for the pipeline bench: the same chain with no token filter, so
/// the filter's share is readable as the difference.
#[cfg(feature = "embed-ipadic")]
fn bench_pipeline_without_filter(c: &mut Criterion) {
    let text = corpus();
    let tokenizer = Tokenizer::new(segmenter());

    c.bench_function("bench-tag-filters-pipeline-no-filter-ipadic", |b| {
        let mut worker = tokenizer.new_worker();
        b.iter(|| worker.tokenize(&text).unwrap().len())
    });
}

#[cfg(feature = "embed-ipadic")]
criterion_group!(
    benches,
    bench_apply_japanese_stop_tags,
    bench_apply_japanese_keep_tags,
    bench_apply_empty_tag_set,
    bench_pipeline_with_stop_tags,
    bench_pipeline_without_filter,
);

#[cfg(feature = "embed-ipadic")]
criterion_main!(benches);

#[cfg(not(feature = "embed-ipadic"))]
fn main() {
    eprintln!("bench_tag_filters_ipadic requires --features embed-ipadic");
}

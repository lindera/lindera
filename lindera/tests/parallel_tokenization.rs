//! The tokens of a batch on several threads are those of one worker over
//! the same texts, for the embedded dictionaries of the golden tests but
//! UniDic CWJ and UniDic CSJ (#1402).
//!
//! The CI jobs that embed UniDic CWJ and UniDic CSJ run only the tests of
//! lindera that read those dictionaries, since each test binary that embeds
//! one takes about 1.2 GB; a batch runs the same code for every dictionary,
//! so they are left out here.
//!
//! `AnalysisWorkerPool` tokenizes every text as a call of its own. These
//! tests compare every field of every token of a batch on 4 threads with
//! one worker's, in Normal and Decompose modes, with N-best (IPADIC), and
//! with filters that look at more than one character or token: the
//! iteration mark, mapping and Unicode normalization character filters, and
//! the compound word and Korean decompound token filters. The texts are
//! long enough in all for every batch to run on several threads, which each
//! test checks.
//!
//! ```sh
//! cargo test -p lindera --features embed-ipadic,embed-ko-dic --test parallel_tokenization
//! ```

#![cfg(all(
    feature = "analysis",
    any(
        feature = "embed-ipadic",
        feature = "embed-unidic",
        feature = "embed-sudachidict",
        feature = "embed-ko-dic",
        feature = "embed-jieba",
    )
))]

use std::num::NonZeroUsize;
use std::path::Path;

use lindera::analysis::character_filter::CharacterFilterLoader;
use lindera::analysis::token_filter::TokenFilterLoader;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::dictionary::{WordId, load_dictionary};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera::token::Token;
use serde_json::{Value, json};

/// Every field of a token: surface, byte range, position, position length,
/// word id and details.
type Fields = (String, usize, usize, usize, usize, WordId, Vec<String>);

/// Filters, each given as its kind and its arguments: the character filters,
/// then the token filters.
type Filters = (Vec<(&'static str, Value)>, Vec<(&'static str, Value)>);

/// The most threads a batch runs on.
fn four() -> NonZeroUsize {
    NonZeroUsize::new(4).expect("4 is not 0")
}

/// The fields of `tokens`.
fn fields(tokens: Vec<Token<'_>>) -> Vec<Fields> {
    tokens
        .into_iter()
        .map(|mut token| {
            let details = token.details().iter().map(|d| d.to_string()).collect();
            (
                token.surface.to_string(),
                token.byte_start,
                token.byte_end,
                token.position,
                token.position_length,
                token.word_id,
                details,
            )
        })
        .collect()
}

/// The first `lines` lines of a file of `resources/`, then `extra`.
#[allow(dead_code)]
fn resource_lines(file: &str, lines: usize, extra: &[&str]) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join(file);
    let text = std::fs::read_to_string(&path).expect("the resource should be readable");
    text.lines()
        .take(lines)
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

/// Lines of `bocchan.txt`, the Japanese texts of the golden tests, and texts
/// for the iteration mark and mapping filters.
#[allow(dead_code)]
fn japanese_texts() -> Vec<String> {
    resource_lines(
        "bocchan.txt",
        200,
        &[
            "関西国際空港限定トートバッグ",
            "すもももももももものうち",
            "日本語の形態素解析を行うことができます。",
            "Linderaは形態素解析エンジンです。ユーザー辞書も利用可能です。",
            "１９８４年と1984年、ＡＢＣとabc。",
            "羽田空港から東京タワーまでタクシーで３０分です。",
            "ゝ学問のすゝめ、時々、こゝろ",
            "リンデラは３２コアで動く",
        ],
    )
}

/// Lines of `sangnoksu.txt` and the Korean texts of the golden tests.
#[allow(dead_code)]
fn korean_texts() -> Vec<String> {
    resource_lines(
        "sangnoksu.txt",
        1200,
        &[
            "한국어의형태소해석을실시할수있습니다.",
            "아버지가방에들어가신다",
            "대한민국의 수도는 서울입니다.",
            "서울 시 에서 출발",
        ],
    )
}

/// The Chinese texts of the golden tests, repeated.
#[allow(dead_code)]
fn chinese_texts() -> Vec<String> {
    [
        "可以进行中文形态学分析。",
        "北京是中华人民共和国的首都。",
        "我喜欢吃苹果和香蕉。",
    ]
    .iter()
    .cycle()
    .take(2400)
    .map(|text| text.to_string())
    .collect()
}

/// The filters of the Japanese dictionaries.
#[allow(dead_code)]
fn japanese_filters() -> Filters {
    (
        vec![
            (
                "japanese_iteration_mark",
                json!({"normalize_kanji": true, "normalize_kana": true}),
            ),
            ("mapping", json!({"mapping": {"リンデラ": "Lindera"}})),
            ("unicode_normalize", json!({"kind": "nfkc"})),
        ],
        vec![(
            "japanese_compound_word",
            json!({"tags": ["名詞,数"], "new_tag": "名詞,数"}),
        )],
    )
}

/// The filters of ko-dic.
#[allow(dead_code)]
fn korean_filters() -> Filters {
    (
        vec![("unicode_normalize", json!({"kind": "nfkc"}))],
        vec![("korean_decompound", json!({"mode": "mixed"}))],
    )
}

/// The filters of Jieba.
#[allow(dead_code)]
fn chinese_filters() -> Filters {
    (
        vec![
            ("mapping", json!({"mapping": {"北京": "北平"}})),
            ("unicode_normalize", json!({"kind": "nfkc"})),
        ],
        vec![],
    )
}

/// A tokenizer over the embedded dictionary `uri` in `mode`, with the
/// filters `(character filters, token filters)`, each given as its kind
/// and its arguments.
fn tokenizer(uri: &str, mode: Mode, filters: &Filters) -> Tokenizer {
    let dictionary = load_dictionary(uri).expect("the embedded dictionary should load");
    let mut tokenizer = Tokenizer::new(Segmenter::new(mode, dictionary, None));
    for (kind, args) in &filters.0 {
        let filter = CharacterFilterLoader::load_from_value(kind, args)
            .expect("the character filter should load");
        tokenizer.append_character_filter(filter);
    }
    for (kind, args) in &filters.1 {
        let filter =
            TokenFilterLoader::load_from_value(kind, args).expect("the token filter should load");
        tokenizer.append_token_filter(filter);
    }
    tokenizer
}

/// Asserts that a batch on up to 4 threads gives one worker's tokens, and
/// that it ran on more than one thread.
fn assert_batch_matches_one_worker(tokenizer: &Tokenizer, texts: &[String]) {
    let mut worker = tokenizer.new_worker();
    let expected: Vec<Vec<Fields>> = texts
        .iter()
        .map(|text| fields(worker.tokenize(text).expect("tokenize should succeed")))
        .collect();
    let mut pool = tokenizer.new_worker_pool();
    let actual = pool
        .tokenize_batch(texts, four(), fields)
        .expect("the batch should succeed");
    assert!(pool.worker_count() > 1, "the batch ran on one thread");
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert!(
            actual == expected,
            "text {index} differs: {:?}",
            texts[index]
        );
    }
}

/// Defines, for the dictionary `$uri` behind `$feature`, the tests of a
/// batch of `$texts` in Normal mode, in Decompose mode, and in Normal mode
/// with `$filters`.
macro_rules! parallel_tests {
    ($feature:literal, $mod_name:ident, $uri:literal, $texts:expr, $filters:expr) => {
        #[cfg(feature = $feature)]
        mod $mod_name {
            use super::*;

            /// A batch in Normal mode, without filters.
            #[test]
            fn normal() {
                let tokenizer = tokenizer($uri, Mode::Normal, &(vec![], vec![]));
                assert_batch_matches_one_worker(&tokenizer, &$texts);
            }

            /// A batch in Decompose mode, without filters.
            #[test]
            fn decompose() {
                let tokenizer =
                    tokenizer($uri, Mode::Decompose(Penalty::default()), &(vec![], vec![]));
                assert_batch_matches_one_worker(&tokenizer, &$texts);
            }

            /// A batch in Normal mode, with the filters.
            #[test]
            fn with_filters() {
                let tokenizer = tokenizer($uri, Mode::Normal, &$filters);
                assert_batch_matches_one_worker(&tokenizer, &$texts);
            }
        }
    };
}

parallel_tests!(
    "embed-ipadic",
    ipadic,
    "embedded://ipadic",
    japanese_texts(),
    japanese_filters()
);
parallel_tests!(
    "embed-unidic",
    unidic,
    "embedded://unidic",
    japanese_texts(),
    japanese_filters()
);
parallel_tests!(
    "embed-sudachidict",
    sudachidict,
    "embedded://sudachidict",
    japanese_texts(),
    japanese_filters()
);
parallel_tests!(
    "embed-ko-dic",
    ko_dic,
    "embedded://ko-dic",
    korean_texts(),
    korean_filters()
);
parallel_tests!(
    "embed-jieba",
    jieba,
    "embedded://jieba",
    chinese_texts(),
    chinese_filters()
);

/// The N-best batch gives one worker's results (IPADIC, with the filters).
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_nbest() {
    let tokenizer = tokenizer("embedded://ipadic", Mode::Normal, &japanese_filters());
    let texts = japanese_texts();
    let mut worker = tokenizer.new_worker();
    let expected: Vec<Vec<(Vec<Fields>, i64)>> = texts
        .iter()
        .map(|text| {
            worker
                .tokenize_nbest(text, 3, true, Some(5000))
                .expect("tokenize_nbest should succeed")
                .into_iter()
                .map(|(tokens, cost)| (fields(tokens), cost))
                .collect()
        })
        .collect();
    let mut pool = tokenizer.new_worker_pool();
    let actual = pool
        .tokenize_nbest_batch(&texts, 3, true, Some(5000), four(), |results| {
            results
                .into_iter()
                .map(|(tokens, cost)| (fields(tokens), cost))
                .collect::<Vec<_>>()
        })
        .expect("the N-best batch should succeed");
    assert!(pool.worker_count() > 1, "the batch ran on one thread");
    assert!(
        actual == expected,
        "the N-best batch differs from one worker"
    );
}

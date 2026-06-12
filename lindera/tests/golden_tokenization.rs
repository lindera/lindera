//! Golden (snapshot) tests for tokenization output.
//!
//! These tests pin the exact tokenization results (surface form, byte
//! offsets, token positions and details) for each embedded dictionary in
//! both `Normal` and `Decompose` modes. They serve as a safety net for
//! refactoring: any change to the tokenizer, segmenter, Viterbi lattice,
//! dictionary loading or filter pipeline that alters output is caught here.
//!
//! Snapshots are stored in `tests/snapshots/`. To update them after an
//! intentional behavior change, run:
//!
//! ```sh
//! INSTA_UPDATE=always cargo test -p lindera --features embed-ipadic,embed-ko-dic --test golden_tokenization
//! cargo insta review  # or inspect the diff manually
//! ```

//! Snapshots currently exist for IPADIC, ko-dic and Jieba. Tests for the
//! remaining embedded dictionaries (UniDic, IPADIC NEologd, CC-CEDICT) can
//! be added with the same `golden_tests!` macro once snapshots have been
//! generated in an environment where those dictionaries can be downloaded
//! and embedded.

#![cfg(any(
    feature = "embed-ipadic",
    feature = "embed-ko-dic",
    feature = "embed-jieba",
))]

use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;

#[allow(dead_code)]
const JAPANESE_TEXTS: &[&str] = &[
    "関西国際空港限定トートバッグ",
    "すもももももももものうち",
    "日本語の形態素解析を行うことができます。",
    "Linderaは形態素解析エンジンです。ユーザー辞書も利用可能です。",
    "１９８４年と1984年、ＡＢＣとabc。",
    "羽田空港から東京タワーまでタクシーで３０分です。",
];

#[allow(dead_code)]
const KOREAN_TEXTS: &[&str] = &[
    "한국어의형태소해석을실시할수있습니다.",
    "아버지가방에들어가신다",
    "대한민국의 수도는 서울입니다.",
];

#[allow(dead_code)]
const CHINESE_TEXTS: &[&str] = &[
    "可以进行中文形态学分析。",
    "北京是中华人民共和国的首都。",
    "我喜欢吃苹果和香蕉。",
];

/// Builds a tokenizer backed by an embedded dictionary.
#[allow(dead_code)]
fn tokenizer(uri: &str, mode: Mode) -> Tokenizer {
    let dictionary =
        lindera::dictionary::load_dictionary(uri).expect("embedded dictionary should load");
    Tokenizer::new(Segmenter::new(mode, dictionary, None))
}

/// Renders tokenization results into a stable, human-readable text form:
/// one line per token with surface, byte range, position, position length
/// and dictionary details.
#[allow(dead_code)]
fn render(tokenizer: &Tokenizer, texts: &[&str]) -> String {
    let mut out = String::new();
    for text in texts {
        out.push_str("## ");
        out.push_str(text);
        out.push('\n');
        let mut tokens = tokenizer
            .tokenize(text)
            .expect("tokenization should succeed");
        for token in tokens.iter_mut() {
            let details = token.details().join(",");
            out.push_str(&format!(
                "{}\t{}..{}\t{}:{}\t{}\n",
                token.surface,
                token.byte_start,
                token.byte_end,
                token.position,
                token.position_length,
                details
            ));
        }
        out.push('\n');
    }
    out
}

macro_rules! golden_tests {
    ($feature:literal, $mod_name:ident, $uri:literal, $texts:expr) => {
        #[cfg(feature = $feature)]
        mod $mod_name {
            use super::*;

            #[test]
            fn normal() {
                let tokenizer = tokenizer($uri, Mode::Normal);
                insta::assert_snapshot!(
                    concat!(stringify!($mod_name), "_normal"),
                    render(&tokenizer, $texts)
                );
            }

            #[test]
            fn decompose() {
                let tokenizer = tokenizer($uri, Mode::Decompose(Penalty::default()));
                insta::assert_snapshot!(
                    concat!(stringify!($mod_name), "_decompose"),
                    render(&tokenizer, $texts)
                );
            }
        }
    };
}

golden_tests!("embed-ipadic", ipadic, "embedded://ipadic", JAPANESE_TEXTS);
golden_tests!("embed-ko-dic", ko_dic, "embedded://ko-dic", KOREAN_TEXTS);
golden_tests!("embed-jieba", jieba, "embedded://jieba", CHINESE_TEXTS);

/// Pins tokenization output with a user dictionary applied (IPADIC).
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_user_dictionary() {
    use std::fs::File;
    use std::path::PathBuf;

    use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ipadic")
        .join("metadata.json");
    let metadata: Metadata =
        serde_json::from_reader(File::open(metadata_file).expect("metadata.json should open"))
            .expect("metadata.json should parse");

    let userdic_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("user_dict")
        .join("ipadic_simple_userdic.csv");

    let dictionary = load_dictionary("embedded://ipadic").expect("embedded dictionary should load");
    let user_dictionary = load_user_dictionary(userdic_file.to_str().unwrap(), &metadata)
        .expect("user dictionary should load");
    let tokenizer = Tokenizer::new(Segmenter::new(
        Mode::Normal,
        dictionary,
        Some(user_dictionary),
    ));

    let texts = &["東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"];
    insta::assert_snapshot!("ipadic_user_dictionary", render(&tokenizer, texts));
}

/// Pins N-best tokenization output (IPADIC).
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_nbest() {
    let tokenizer = tokenizer("embedded://ipadic", Mode::Normal);

    let text = "関西国際空港限定トートバッグ";
    let results = tokenizer
        .tokenize_nbest(text, 3, false, None)
        .expect("tokenization should succeed");

    let mut out = String::new();
    for (i, (mut tokens, cost)) in results.into_iter().enumerate() {
        out.push_str(&format!("## candidate {i} (cost: {cost})\n"));
        for token in tokens.iter_mut() {
            let details = token.details().join(",");
            out.push_str(&format!(
                "{}\t{}..{}\t{}\n",
                token.surface, token.byte_start, token.byte_end, details
            ));
        }
        out.push('\n');
    }
    insta::assert_snapshot!("ipadic_nbest", out);
}

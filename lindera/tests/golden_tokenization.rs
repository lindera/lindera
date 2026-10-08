//! Golden (snapshot) tests for tokenization output.
//!
//! These tests pin the exact tokenization results (surface form, byte
//! offsets, token positions and details) for each embedded dictionary in
//! both `Normal` and `Decompose` modes. They serve as a safety net for
//! refactoring: any change to the segmenter, Viterbi lattice or
//! dictionary loading that alters output is caught here.
//!
//! Snapshots are stored in `tests/snapshots/`. To update them after an
//! intentional behavior change, run:
//!
//! ```sh
//! INSTA_UPDATE=always cargo test -p lindera --features embed-ipadic,embed-ko-dic --test golden_tokenization
//! cargo insta review  # or inspect the diff manually
//! ```

//! Snapshots currently exist for IPADIC, UniDic, SudachiDict, ko-dic and
//! Jieba. Tests for the remaining embedded dictionaries (IPADIC NEologd,
//! CC-CEDICT) can be added with the same `golden_tests!` macro once snapshots
//! have been generated in an environment where those dictionaries can be
//! downloaded and embedded.

#![cfg(any(
    feature = "embed-ipadic",
    feature = "embed-unidic",
    feature = "embed-sudachidict",
    feature = "embed-ko-dic",
    feature = "embed-jieba",
))]

use std::borrow::Cow;

use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;

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
    // A particle and an ending after a space: pins the left-space penalty
    // that ko-dic applies by default (`시` reads as NNG, not EP) and the
    // whitespace skipping that lets `에서` connect to `시` (JKB, not NNG).
    "서울 시 에서 출발",
];

#[allow(dead_code)]
const CHINESE_TEXTS: &[&str] = &[
    "可以进行中文形态学分析。",
    "北京是中华人民共和国的首都。",
    "我喜欢吃苹果和香蕉。",
];

/// Builds a segmenter backed by an embedded dictionary.
#[allow(dead_code)]
fn segmenter(uri: &str, mode: Mode) -> Segmenter {
    let dictionary =
        lindera::dictionary::load_dictionary(uri).expect("embedded dictionary should load");
    Segmenter::new(mode, dictionary, None)
}

/// Renders tokenization results into a stable, human-readable text form:
/// one line per token with surface, byte range, position, position length
/// and dictionary details.
#[allow(dead_code)]
fn render(segmenter: &Segmenter, texts: &[&str]) -> String {
    let mut out = String::new();
    for text in texts {
        out.push_str("## ");
        out.push_str(text);
        out.push('\n');
        let mut tokens = segmenter
            .segment(Cow::Borrowed(text))
            .expect("segmentation should succeed");
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
                let segmenter = segmenter($uri, Mode::Normal);
                insta::assert_snapshot!(
                    concat!(stringify!($mod_name), "_normal"),
                    render(&segmenter, $texts)
                );
            }

            #[test]
            fn decompose() {
                let segmenter = segmenter($uri, Mode::Decompose(Penalty::default()));
                insta::assert_snapshot!(
                    concat!(stringify!($mod_name), "_decompose"),
                    render(&segmenter, $texts)
                );
            }
        }
    };
}

golden_tests!("embed-ipadic", ipadic, "embedded://ipadic", JAPANESE_TEXTS);
golden_tests!("embed-unidic", unidic, "embedded://unidic", JAPANESE_TEXTS);
golden_tests!(
    "embed-sudachidict",
    sudachidict,
    "embedded://sudachidict",
    JAPANESE_TEXTS
);
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
    let segmenter = Segmenter::new(Mode::Normal, dictionary, Some(user_dictionary));

    let texts = &["東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です"];
    insta::assert_snapshot!("ipadic_user_dictionary", render(&segmenter, texts));
}

/// Renders N-best results as one block per candidate: its cost, then one
/// `surface<TAB>start..end<TAB>details` line per token.
#[cfg(feature = "embed-ipadic")]
fn render_nbest(results: Vec<(Vec<lindera::token::Token>, i64)>) -> String {
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
    out
}

/// Pins N-best tokenization output (IPADIC).
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_nbest() {
    let segmenter = segmenter("embedded://ipadic", Mode::Normal);

    let text = "関西国際空港限定トートバッグ";
    let results = segmenter
        .segment_nbest(Cow::Borrowed(text), 3, false, None)
        .expect("segmentation should succeed");
    insta::assert_snapshot!("ipadic_nbest", render_nbest(results));
}

/// Pins N-best output over several sentences (IPADIC, #1097, #1096): every
/// candidate segments the whole input, and the context is carried across
/// `、` and `。`, so the candidates and their costs are those of one lattice
/// over the line.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_nbest_multi_sentence() {
    use lindera::dictionary::viterbi::{Lattice, LatticeOptions};

    let segmenter = segmenter("embedded://ipadic", Mode::Normal);

    let text = "東京、です。関西国際空港へ行く";
    let results = segmenter
        .segment_nbest(Cow::Borrowed(text), 5, false, None)
        .expect("segmentation should succeed");

    // The same costs and segmentations as one lattice over the whole line
    // (the line has no whitespace and no word across a cut).
    let dictionary = &segmenter.dictionary;
    let mut lattice = Lattice::default();
    lattice.set_text_nbest_with_options(
        &dictionary.prefix_dictionary,
        &None,
        &dictionary.character_definition,
        &dictionary.unknown_dictionary,
        &dictionary.connection_cost_matrix,
        text,
        &LatticeOptions::new(&Mode::Normal),
    );
    let unsplit = lattice.nbest_tokens_offset(5, false, None);
    let carried: Vec<_> = results
        .iter()
        .map(|(tokens, cost)| {
            let offsets: Vec<_> = tokens
                .iter()
                .map(|token| (token.byte_start, token.byte_end, token.word_id))
                .collect();
            (offsets, *cost)
        })
        .collect();
    assert_eq!(carried, unsplit);

    insta::assert_snapshot!("ipadic_nbest_multi_sentence", render_nbest(results));
}

/// Regression test #1016: Apply the Decompose penalty when connecting a compound to EOS.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_decompose_compound_at_eos() {
    let segmenter = segmenter("embedded://ipadic", Mode::Decompose(Penalty::default()));

    for (text, expected) in [
        ("東京大学", vec!["東京", "大学"]),
        ("関西国際空港", vec!["関西", "国際", "空港"]),
    ] {
        let surfaces = segmenter
            .segment(Cow::Borrowed(text))
            .expect("segmentation should succeed")
            .into_iter()
            .map(|token| token.surface.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(surfaces, expected);

        let nbest_surfaces = segmenter
            .segment_nbest(Cow::Borrowed(text), 1, false, None)
            .expect("N-best segmentation should succeed")
            .into_iter()
            .next()
            .expect("at least one N-best path should exist")
            .0
            .into_iter()
            .map(|token| token.surface.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(nbest_surfaces, expected);
    }
}

/// Segments `text` and returns each token's surface with its first `fields`
/// detail fields joined by commas.
#[allow(dead_code)]
fn surfaces_and_details(segmenter: &Segmenter, text: &str, fields: usize) -> Vec<(String, String)> {
    segmenter
        .segment(Cow::Borrowed(text))
        .expect("segmentation should succeed")
        .iter_mut()
        .map(|token| {
            let details = token.details()[..fields].join(",");
            (token.surface.to_string(), details)
        })
        .collect()
}

/// Regression test #1094: the full-width space is IPADIC's `記号,空白`
/// entry (the dictionary builder used to trim the surface and drop it), and
/// `ルーマニア` is no longer taken over by its trailing-U+3000 variant.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_whitespace_entries() {
    let segmenter = segmenter("embedded://ipadic", Mode::Normal);

    assert_eq!(
        surfaces_and_details(&segmenter, "東京\u{3000}都", 2),
        vec![
            ("東京".to_string(), "名詞,固有名詞".to_string()),
            ("\u{3000}".to_string(), "記号,空白".to_string()),
            ("都".to_string(), "名詞,一般".to_string()),
        ]
    );

    let mut tokens = segmenter
        .segment(Cow::Borrowed("ルーマニア"))
        .expect("segmentation should succeed");
    assert_eq!(tokens.len(), 1);
    // details[6] is the base form.
    assert_eq!(tokens[0].details()[6], "ルーマニア");
}

/// Regression test #1106: IPADIC entries spelled with U+2015 (HORIZONTAL
/// BAR) match text spelled the same way, as in MeCab. The dictionary builder
/// used to store them under U+2014 (EM DASH), which the text does not use,
/// so `ＣＤ―ＲＯＭ` was split and `――` was an unknown noun.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_dash_entries() {
    let segmenter = segmenter("embedded://ipadic", Mode::Normal);

    let mut tokens = segmenter
        .segment(Cow::Borrowed("ＣＤ\u{2015}ＲＯＭ"))
        .expect("segmentation should succeed");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].surface, "ＣＤ\u{2015}ＲＯＭ");
    // details[6] is the base form, kept as written in the CSV.
    assert_eq!(tokens[0].details()[6], "ＣＤ\u{2015}ＲＯＭ");

    assert_eq!(
        surfaces_and_details(&segmenter, "\u{2015}\u{2015}", 2),
        vec![("\u{2015}\u{2015}".to_string(), "記号,一般".to_string())]
    );

    // No entry is spelled with U+2014, so the text is split, as in MeCab.
    assert_eq!(
        surfaces_and_details(&segmenter, "ＣＤ\u{2014}ＲＯＭ", 2),
        vec![
            ("ＣＤ".to_string(), "名詞,一般".to_string()),
            ("\u{2014}".to_string(), "名詞,サ変接続".to_string()),
            ("ＲＯＭ".to_string(), "名詞,固有名詞".to_string()),
        ]
    );
}

/// Regression test #1131: IPADIC's two `狡い` entries tie (same context ids
/// and cost; only the reading and the pronunciation differ), and the first
/// CSV row wins, as in MeCab: `ズルイ`, not `コスイ`. The lattice used to
/// add a surface's entries last row first and so picked the last row.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_tied_entries() {
    let segmenter = segmenter("embedded://ipadic", Mode::Normal);

    let mut tokens = segmenter
        .segment(Cow::Borrowed("あいつは狡い"))
        .expect("segmentation should succeed");
    let surfaces: Vec<&str> = tokens.iter().map(|token| token.surface.as_ref()).collect();
    assert_eq!(surfaces, vec!["あいつ", "は", "狡い"]);
    // details[7] is the reading and details[8] the pronunciation.
    let details = tokens[2].details();
    assert_eq!(details[7], "ズルイ");
    assert_eq!(details[8], "ズルイ");
}

/// Regression test #1131: the first N-best result is the output of
/// `segment` also when several paths cost the same. IPADIC has six tied
/// `大平山` entries (`Noun.proper.csv`) and eight tied `西河内` entries; the
/// N-best search, which ordered its queue by cost alone, used to return
/// another of them first (`オオヒラヤマ` instead of `オオヒラサン`,
/// `ニシゴウド` instead of `ニシガワウチ`). The line with `、` covers the
/// search of a segment whose context is carried across the cut. In
/// Decompose mode only `大平山` stays one word, so only it ties there.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_tied_entries_nbest() {
    /// Each token's surface and full details.
    fn surfaces_and_all_details(tokens: &mut [lindera::token::Token]) -> Vec<(String, String)> {
        tokens
            .iter_mut()
            .map(|token| (token.surface.to_string(), token.details().join(",")))
            .collect()
    }

    let cases = [
        (Mode::Normal, &["大平山", "西河内", "大平山、西河内"][..]),
        (Mode::Decompose(Penalty::default()), &["大平山"][..]),
    ];
    for (mode, texts) in cases {
        let segmenter = segmenter("embedded://ipadic", mode);

        let mut results = segmenter
            .segment_nbest(Cow::Borrowed("あいつは狡い"), 3, false, None)
            .expect("segmentation should succeed");
        let first = &mut results[0].0;
        let surfaces: Vec<&str> = first.iter().map(|token| token.surface.as_ref()).collect();
        assert_eq!(surfaces, vec!["あいつ", "は", "狡い"]);
        // details[7] is the reading.
        assert_eq!(first[2].details()[7], "ズルイ");

        for text in texts {
            let case = format!("{text} {:?}", segmenter.mode);
            let mut expected = segmenter
                .segment(Cow::Borrowed(text))
                .expect("segmentation should succeed");
            let mut results = segmenter
                .segment_nbest(Cow::Borrowed(text), 3, false, None)
                .expect("segmentation should succeed");
            let costs: Vec<i64> = results.iter().map(|(_, cost)| *cost).collect();
            assert!(costs.windows(2).all(|pair| pair[0] <= pair[1]), "{case}");
            assert_eq!(costs.len(), 3, "{case}");
            assert_eq!(costs[0], costs[1], "{case}: the first two paths tie");
            assert_eq!(
                surfaces_and_all_details(&mut results[0].0),
                surfaces_and_all_details(&mut expected),
                "{case}"
            );
        }
    }
}

/// Asserts that `text` segments into `expected` and that the first N-best
/// result does too, tied with the second (#1135).
#[cfg(any(feature = "embed-ipadic", feature = "embed-ko-dic"))]
fn assert_cross_start_tie(segmenter: &Segmenter, text: &str, expected: &[&str]) {
    let tokens = segmenter
        .segment(Cow::Borrowed(text))
        .expect("segmentation should succeed");
    let surfaces: Vec<&str> = tokens.iter().map(|token| token.surface.as_ref()).collect();
    assert_eq!(surfaces, expected, "{text}");

    let results = segmenter
        .segment_nbest(Cow::Borrowed(text), 2, false, None)
        .expect("segmentation should succeed");
    let first: Vec<&str> = results[0]
        .0
        .iter()
        .map(|token| token.surface.as_ref())
        .collect();
    assert_eq!(first, expected, "{text}: the first N-best result");
    assert_eq!(results[0].1, results[1].1, "{text}: the two paths tie");
}

/// Regression test #1135: of two segmentations of equal cost that end at
/// the same position, the one whose last word starts later wins, as in
/// MeCab: `窒扶 / 斯` over `窒 / 扶斯` (unknown words), and `フリー /
/// ホイール / ダイオード` over the one unknown word
/// `フリーホイールダイオード`. The lattice used to keep the
/// earlier-starting word.
#[cfg(feature = "embed-ipadic")]
#[test]
fn ipadic_cross_start_ties() {
    let segmenter = segmenter("embedded://ipadic", Mode::Normal);
    assert_cross_start_tie(&segmenter, "腸窒扶斯", &["腸", "窒扶", "斯"]);
    assert_cross_start_tie(
        &segmenter,
        "にフリーホイールダイオードや",
        &["に", "フリー", "ホイール", "ダイオード", "や"],
    );
}

/// Regression test #1135: `차 / 나` (a noun and a particle) and `차나` (one
/// noun) cost the same before `마셔`, and the later-starting `나` wins, as
/// in MeCab, with ko-dic's default left-space penalty.
#[cfg(feature = "embed-ko-dic")]
#[test]
fn ko_dic_cross_start_ties() {
    let segmenter = segmenter("embedded://ko-dic", Mode::Normal);
    assert_cross_start_tie(&segmenter, "차나 마셔", &["차", "나", "마셔"]);
}

/// Regression test #1094: the full-width space is UniDic's `空白` entry.
#[cfg(feature = "embed-unidic")]
#[test]
fn unidic_whitespace_entries() {
    let segmenter = segmenter("embedded://unidic", Mode::Normal);

    let tokens = surfaces_and_details(&segmenter, "東京\u{3000}都", 1);
    assert_eq!(tokens[1], ("\u{3000}".to_string(), "空白".to_string()));
}

/// Regression test #1094: SudachiDict's half-width space entry (display
/// surface `" "`, part of speech `空白`) is used instead of the `SPACE`
/// unknown word, whose display surface is `*`.
#[cfg(feature = "embed-sudachidict")]
#[test]
fn sudachidict_whitespace_entries() {
    let segmenter = segmenter("embedded://sudachidict", Mode::Normal).keep_whitespace(true);

    let tokens = surfaces_and_details(&segmenter, "a b", 2);
    assert_eq!(tokens[1], (" ".to_string(), " ,空白".to_string()));
}

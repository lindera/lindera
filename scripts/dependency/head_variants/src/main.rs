//! The agreement of the head word with the content head of a corpus, for the
//! rule of CaboCha and for each addition to it, up to `Schema::ipadic()`.
//!
//! Each file is converted once per variant with `corpus::convert`, and the
//! counts of its report are printed: the file, the variant, the bunsetsu
//! whose head word is the content head, the bunsetsu measured, the share.
//!
//! Usage: head_variants <CoNLL-U file>...

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufReader};

use lindera_dependency::corpus::{self, DictionaryMorpheme, TokenizerError};
use lindera_dependency::{Pattern, Schema};
use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

/// The variants, each one the one before with one addition.
///
/// The last one is checked to be `Schema::ipadic()`: when the schema
/// changes, this list has to follow.
fn variants() -> Vec<(&'static str, Schema)> {
    let ipadic = Schema::ipadic();
    let mut variants = Vec::new();

    // CaboCha: its six function word patterns, and 記号 by the part of
    // speech only.
    let mut schema = ipadic.clone();
    schema.function_words.truncate(6);
    schema.not_heads.clear();
    for marks in [
        &mut schema.symbols.others,
        &mut schema.symbols.commas,
        &mut schema.symbols.periods,
        &mut schema.symbols.open_brackets,
        &mut schema.symbols.close_brackets,
    ] {
        marks.characters.clear();
    }
    variants.push(("the rule of CaboCha for IPA", schema.clone()));

    schema.symbols = ipadic.symbols.clone();
    variants.push(("+ symbols by their surface", schema.clone()));

    schema.function_words = ipadic.function_words.clone();
    variants.push(("+ する, できる are function words", schema.clone()));

    schema.not_heads.push(Pattern::new(&["名詞", "非自立"]));
    variants.push(("+ 名詞,非自立 is not a head", schema.clone()));

    schema
        .not_heads
        .push(Pattern::new(&["名詞", "接尾", "形容動詞語幹"]));
    variants.push(("+ the suffix 的 is not a head", schema.clone()));

    schema
        .not_heads
        .push(Pattern::new(&["名詞", "接尾", "特殊"]));
    variants.push(("+ the suffix さ is not a head", schema.clone()));

    schema
        .not_heads
        .push(Pattern::new(&["動詞", "自立"]).after(Pattern::new(&["助詞"])));
    variants.push(("+ a verb after a particle is not a head", schema.clone()));

    assert_eq!(schema, ipadic, "the last variant is Schema::ipadic()");
    variants
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    // The default settings, as `lindera-dependency convert` has them.
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenize = |text: &str| -> Result<Vec<DictionaryMorpheme>, TokenizerError> {
        let mut tokens = segmenter.segment(Cow::Borrowed(text))?;
        Ok(tokens
            .iter_mut()
            .map(|token| {
                let features = token.details_iter().map(str::to_string).collect();
                DictionaryMorpheme::new(token.surface.to_string(), features)
            })
            .collect())
    };

    for path in std::env::args().skip(1) {
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        for (label, schema) in variants() {
            let input = BufReader::new(File::open(&path)?);
            let report = corpus::convert(input, &mut io::sink(), &schema, &tokenize)?;
            println!(
                "{name}\t{label}\t{}\t{}\t{:.2}%",
                report.heads_agree,
                report.heads,
                100.0 * report.heads_agree as f64 / report.heads as f64
            );
        }
    }
    Ok(())
}

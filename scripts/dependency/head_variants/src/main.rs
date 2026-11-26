//! The agreement of the head word with the content head of a corpus, for the
//! rule of CaboCha and for each addition to it, up to the schema of the
//! dictionary: `Schema::ipadic()` or `Schema::unidic()`.
//!
//! Each file is converted once per variant with `corpus::convert`, and the
//! counts of its report are printed: the file, the variant, the bunsetsu
//! whose head word is the content head, the bunsetsu measured, the share.
//!
//! Usage: head_variants [--dict <URI or path>] <CoNLL-U file>...
//!
//! The dictionary is the embedded IPADIC when none is given. The variants
//! are those of the schema of the dictionary, by its name.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufReader};

use lindera_dependency::corpus::{self, DictionaryMorpheme, TokenizerError};
use lindera_dependency::{Marks, Pattern, Schema};
use lindera_segmenter::dictionary::load_dictionary;
use lindera_segmenter::mode::Mode;
use lindera_segmenter::segmenter::Segmenter;

/// The variants for IPADIC, each one the one before with one addition.
///
/// The last one is checked to be `Schema::ipadic()`: when the schema
/// changes, this list has to follow.
fn ipadic_variants() -> Vec<(&'static str, Schema)> {
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

/// The variants for UniDic, each one the one before with one addition.
///
/// The additions are the patterns of `Schema::unidic()` in their order; the
/// numbers of the patterns are checked, and the last variant is checked to
/// be the schema: when the schema changes, this list has to follow. After
/// it come the changes that were measured and not taken.
fn unidic_variants() -> Vec<(&'static str, Schema)> {
    let unidic = Schema::unidic();
    assert_eq!(
        (unidic.function_words.len(), unidic.not_heads.len()),
        (7, 7),
        "the patterns of Schema::unidic() are the ones of this list"
    );
    let mut variants = Vec::new();

    // CaboCha: a particle, an auxiliary, and a suffix that is not nominal.
    let mut schema = unidic.clone();
    schema.function_words.truncate(5);
    schema.not_heads.clear();
    variants.push(("the rule of CaboCha for UniDic", schema.clone()));

    schema.function_words = unidic.function_words.clone();
    variants.push(("+ 非自立可能 is a function word", schema.clone()));

    // The patterns of what is not a head, by the number each addition has.
    let additions = [
        ("+ a verb after に, と or を is not a head", 1),
        ("+ 助動詞語幹 is not a head", 3),
        ("+ a formal noun after a predicate is not a head", 6),
        ("+ a nominal suffix after an adjective is not a head", 7),
    ];
    for (label, count) in additions {
        schema.not_heads = unidic.not_heads[..count].to_vec();
        variants.push((label, schema.clone()));
    }

    assert_eq!(schema, unidic, "the last variant is Schema::unidic()");

    // What was measured and not taken: `Schema::unidic()` with one change.
    let suffix = || Pattern::new(&["接尾辞", "名詞的"]);
    let mut anywhere = unidic.clone();
    anywhere.not_heads.push(suffix());
    variants.push((
        "not taken: a nominal suffix is not a head anywhere",
        anywhere,
    ));

    let mut after_number = unidic.clone();
    after_number
        .not_heads
        .push(suffix().after(Pattern::new(&["名詞", "数詞"])));
    variants.push((
        "not taken: a nominal suffix after a number is not a head",
        after_number,
    ));

    let mut letters = unidic.clone();
    letters.symbols.others = Marks::new(&[&["空白"], &["補助記号"], &["記号", "一般"]], "");
    variants.push(("not taken: 記号,文字 is not a symbol", letters));

    variants
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    let uri = if arguments.first().is_some_and(|first| first == "--dict") {
        if arguments.len() < 2 {
            return Err("--dict needs a dictionary".into());
        }
        arguments.remove(0);
        arguments.remove(0)
    } else {
        "embedded://ipadic".to_string()
    };
    let dictionary = load_dictionary(&uri)?;
    let variants = match dictionary.metadata.name.as_str() {
        "ipadic" => ipadic_variants(),
        "unidic" | "unidic-cwj" | "unidic-csj" => unidic_variants(),
        name => return Err(format!("{uri}: there are no variants for the dictionary {name}").into()),
    };
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

    for path in arguments {
        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
        for (label, schema) in &variants {
            let input = BufReader::new(File::open(&path)?);
            let report = corpus::convert(input, &mut io::sink(), schema, &tokenize)?;
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

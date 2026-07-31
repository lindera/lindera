//! Dump per-context-id connection-matrix access frequency over a corpus.
//!
//! Used to build a corpus-frequency context-id remap (the paper's "Mapped" signal),
//! as opposed to the zero-corpus entry-count proxy. Run it against a corpus, then
//! point `LINDERA_CTX_FREQ_FILE` at the output when building the dictionary.
//!
//! ```text
//! cargo run --release --example ctxfreq_dump \
//!   --features embed-unidic,ctxfreq -- embedded://unidic <corpus.txt> <out_freq.txt>
//! ```
//!
//! Requires the `ctxfreq` feature (the counters are compiled out otherwise) plus
//! the `embed-*` feature matching the dictionary URI passed as the first argument.
//!
//! The histogram must be collected against a dictionary built with
//! `connection_id_mapping` **off**, so the counts are in the original
//! context-ID space (collecting on an already-remapped dictionary would
//! produce counts in the remapped space and double-apply the permutation).

#[cfg(feature = "ctxfreq")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::borrow::Cow;
    use std::path::PathBuf;

    use lindera::dictionary::load_dictionary;
    use lindera::mode::Mode;
    use lindera::segmenter::Segmenter;

    const USAGE: &str = "usage: ctxfreq_dump <dictionary-uri> <corpus> <out>";

    let mut args = std::env::args().skip(1);
    let dictionary_uri = args.next().ok_or(USAGE)?;
    let corpus_path = args.next().ok_or(USAGE)?;
    let out_path = args.next().ok_or(USAGE)?;

    let text = std::fs::read_to_string(&corpus_path)?;

    let dictionary = load_dictionary(&dictionary_uri)?;
    // Capture the matrix axis sizes before the dictionary moves into the segmenter,
    // so the dumped histograms are padded to exactly the matrix dimensions.
    let forward_size = dictionary.connection_cost_matrix.forward_size as usize;
    let backward_size = dictionary.connection_cost_matrix.backward_size as usize;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    // A single pass is enough: the histograms accumulate every cost() access.
    let tokens = segmenter.segment(Cow::Borrowed(text.as_str()))?;

    lindera_dictionary::builder::context_id_remap::dump_ctx_freq(
        PathBuf::from(&out_path).as_path(),
        forward_size,
        backward_size,
    )?;

    println!(
        "dictionary={dictionary_uri} corpus={corpus_path} tokens={} axes: forward={forward_size} backward={backward_size} -> {out_path}",
        tokens.len()
    );
    Ok(())
}

#[cfg(not(feature = "ctxfreq"))]
fn main() {
    eprintln!(
        "this example requires --features ctxfreq plus the embed-* feature for the target dictionary (e.g. --features embed-unidic,ctxfreq)"
    );
}

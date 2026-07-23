//! Dump per-context-id connection-matrix access frequency over a corpus.
//!
//! Used to build a corpus-frequency context-id remap (the paper's "Mapped" signal),
//! as opposed to the zero-corpus entry-count proxy. Run it against a corpus, then
//! point `LINDERA_CTX_FREQ_FILE` at the output when building the dictionary.
//!
//! ```text
//! cargo run --release --example ctxfreq_dump \
//!   --features embed-unidic,ctxfreq -- <corpus.txt> <out_freq.txt>
//! ```
//!
//! Requires the `ctxfreq` feature (the counters are compiled out otherwise).

#[cfg(all(feature = "embed-unidic", feature = "ctxfreq"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::borrow::Cow;
    use std::path::PathBuf;

    use lindera::dictionary::load_dictionary;
    use lindera::mode::Mode;
    use lindera::segmenter::Segmenter;

    let mut args = std::env::args().skip(1);
    let corpus_path = args.next().ok_or("usage: ctxfreq_dump <corpus> <out>")?;
    let out_path = args.next().ok_or("usage: ctxfreq_dump <corpus> <out>")?;

    let text = std::fs::read_to_string(&corpus_path)?;

    let dictionary = load_dictionary("embedded://unidic")?;
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
        "corpus={corpus_path} tokens={} axes: forward={forward_size} backward={backward_size} -> {out_path}",
        tokens.len()
    );
    Ok(())
}

#[cfg(not(all(feature = "embed-unidic", feature = "ctxfreq")))]
fn main() {
    eprintln!("this example requires --features embed-unidic,ctxfreq");
}

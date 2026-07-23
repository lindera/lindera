//! Build-time connection-cost context-ID remapping (the "Mapped" technique).
//!
//! The connection-cost matrix reference is the dominant tokenize bottleneck and is
//! cache-miss bound. This module computes two frequency-ordered permutations of the
//! context IDs so that frequently-used matrix cells cluster near the front of each
//! row (and the top of the matrix), improving reference locality. The permutations
//! are applied consistently to the matrix and to every `WordEntry.left_id/right_id`
//! at build time; the runtime `cost()` lookup is unchanged and tokenization output
//! is bit-identical (the remap is a bijective relabeling that preserves every cost).

use std::cmp::Reverse;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;

use encoding_rs::{Encoding, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;
use glob::glob;

use crate::LinderaResult;
use crate::builder::connection_cost_matrix::read_matrix_header;
use crate::dictionary::metadata::Metadata;
use crate::error::LinderaErrorKind;

/// A pair of context-ID permutations, `perm[old_id] = new_id`.
///
/// `left` permutes left-context IDs (the matrix backward/row axis, length
/// `backward_size`) and `right` permutes right-context IDs (the matrix
/// forward/column axis, length `forward_size`).
#[derive(Debug, Clone)]
pub struct ContextIdRemap {
    /// Permutation of left-context IDs (`WordEntry.left_id`, matrix backward axis).
    pub left: Vec<u16>,
    /// Permutation of right-context IDs (`WordEntry.right_id`, matrix forward axis).
    pub right: Vec<u16>,
}

/// Compute the two frequency-ordered context-ID permutations for a dictionary.
///
/// IDs are ranked most-frequent-first and assigned dense small new IDs; ID 0 is
/// pinned to 0 (reserved for BOS/EOS). The frequency signal is chosen in this order:
///
/// 1. `LINDERA_CTX_FREQ_FILE` environment variable (per-build override).
/// 2. `freq_file` — the histogram bundled with the dictionary crate.
/// 3. Fallback: a zero-corpus entry-count proxy (each lexicon row increments its
///    `left_context_id` / `right_context_id` bucket).
///
/// Only corpus-derived frequency (1 or 2) actually improves cache locality; the
/// entry-count fallback measured ~0% because it mis-ranks high-frequency function
/// words, which have few dictionary entries. It is kept as a safe default and logs a
/// warning. Ranking never affects correctness — the remap is a bijective relabeling —
/// so best-effort counting (skipping unparseable/out-of-range IDs) is fine.
///
/// # Arguments
///
/// * `input_dir` - Directory containing `matrix.def` and the lexicon `*.csv` files.
/// * `metadata` - Dictionary metadata (encoding, schema, `flexible_csv`).
/// * `freq_file` - Optional bundled context-ID frequency histogram.
///
/// # Returns
///
/// A [`ContextIdRemap`] whose axes match the `matrix.def` header sizes, or an error
/// if the header is unreadable, the schema lacks the context-ID columns, or an axis
/// exceeds the `u16` ID range.
pub fn compute_context_id_remap(
    input_dir: &Path,
    metadata: &Metadata,
    freq_file: Option<&Path>,
) -> LinderaResult<ContextIdRemap> {
    let (forward_size, backward_size) = read_matrix_header(input_dir, &metadata.encoding)?;

    // Context IDs are stored as u16; a larger axis cannot be represented.
    if forward_size as usize > u16::MAX as usize + 1
        || backward_size as usize > u16::MAX as usize + 1
    {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "connection matrix axis exceeds u16 range: forward={forward_size}, backward={backward_size}"
        )));
    }

    // Corpus-derived frequency (the paper's signal): a per-build env override first,
    // then the histogram bundled with the dictionary crate.
    let env_override = std::env::var_os("LINDERA_CTX_FREQ_FILE").map(std::path::PathBuf::from);
    let corpus_freq: Option<&Path> = env_override.as_deref().or(freq_file);
    if let Some(freq_path) = corpus_freq {
        let (hist_left, hist_right) =
            load_freq_file(freq_path, backward_size as usize, forward_size as usize)?;
        return Ok(ContextIdRemap {
            left: build_perm(&hist_left),
            right: build_perm(&hist_right),
        });
    }

    log::warn!(
        "connection_id_mapping is enabled but no context-id frequency file was found; \
         falling back to the entry-count proxy, which measured ~0% improvement. \
         Bundle a corpus-derived histogram (see the ctxfreq_dump example) for the real gain."
    );

    let left_index = metadata
        .dictionary_schema
        .get_field_index("left_context_id")
        .ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "schema has no left_context_id field; cannot compute context-id remap"
            ))
        })?;
    let right_index = metadata
        .dictionary_schema
        .get_field_index("right_context_id")
        .ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "schema has no right_context_id field; cannot compute context-id remap"
            ))
        })?;

    let mut hist_left = vec![0u64; backward_size as usize];
    let mut hist_right = vec![0u64; forward_size as usize];

    let encoding =
        Encoding::for_label_no_replacement(metadata.encoding.as_bytes()).ok_or_else(|| {
            LinderaErrorKind::Decode
                .with_error(anyhow::anyhow!("Invalid encoding: {}", metadata.encoding))
        })?;

    let pattern = input_dir
        .to_str()
        .map(|p| format!("{p}/*.csv"))
        .ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                "Input directory path contains invalid characters: {input_dir:?}"
            ))
        })?;

    for entry in
        glob(&pattern).map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?
    {
        let path =
            entry.map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;
        let file = File::open(&path)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        // Mirror the prefix-dictionary reader so id columns are located identically.
        let reader: Box<dyn Read> = if encoding == UTF_8 {
            Box::new(file)
        } else {
            Box::new(
                DecodeReaderBytesBuilder::new()
                    .encoding(Some(encoding))
                    .build(file),
            )
        };
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(metadata.flexible_csv)
            .from_reader(reader);

        for result in rdr.records() {
            let record =
                result.map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;
            if let Some(value) = record.get(left_index)
                && let Ok(id) = u16::from_str(value.trim())
                && (id as usize) < hist_left.len()
            {
                hist_left[id as usize] += 1;
            }
            if let Some(value) = record.get(right_index)
                && let Ok(id) = u16::from_str(value.trim())
                && (id as usize) < hist_right.len()
            {
                hist_right[id as usize] += 1;
            }
        }
    }

    Ok(ContextIdRemap {
        left: build_perm(&hist_left),
        right: build_perm(&hist_right),
    })
}

/// Build a single frequency-ordered permutation from an occurrence histogram.
///
/// ID 0 is pinned to 0 (BOS/EOS). The remaining IDs are ranked by the total order
/// `(count desc, id asc)` — a strict total order, so the result is deterministic and
/// reproducible regardless of sort stability — and assigned dense new IDs `1..n`.
/// IDs absent from the lexicon (count 0, e.g. matrix-only IDs) sort last by ID.
///
/// # Arguments
///
/// * `hist` - Per-ID occurrence counts, indexed by old ID; length = axis size.
///
/// # Returns
///
/// A permutation vector `perm` of the same length where `perm[old_id] = new_id`.
fn build_perm(hist: &[u64]) -> Vec<u16> {
    let n = hist.len();
    if n == 0 {
        return Vec::new();
    }
    let mut ids: Vec<usize> = (1..n).collect();
    ids.sort_by_key(|&id| (Reverse(hist[id]), id));
    let mut perm = vec![0u16; n];
    for (rank, &old) in ids.iter().enumerate() {
        perm[old] = (rank + 1) as u16;
    }
    perm
}

/// Load corpus-derived context-ID access histograms produced by [`dump_ctx_freq`].
///
/// Format: a header line `<backward_size> <forward_size>`, then one whitespace
/// separated line of left-context counts, then one of right-context counts. Both
/// are padded with zeros (or truncated) to the axis sizes taken from `matrix.def`,
/// so IDs never seen in the corpus rank last.
///
/// # Arguments
///
/// * `path` - Histogram file written by the `ctxfreq` instrumentation.
/// * `backward_size` - Left-context axis size from the matrix header.
/// * `forward_size` - Right-context axis size from the matrix header.
///
/// # Returns
///
/// `(hist_left, hist_right)` sized exactly to the two axes.
fn load_freq_file(
    path: &Path,
    backward_size: usize,
    forward_size: usize,
) -> LinderaResult<(Vec<u64>, Vec<u64>)> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!(
                "Failed to read context-id frequency file: {path:?}"
            ))
    })?;
    let mut lines = content.lines();
    // Header line is informational; the axis sizes come from matrix.def.
    lines.next();
    let parse_line = |line: Option<&str>| -> Vec<u64> {
        line.map(|l| {
            l.split_whitespace()
                .map(|s| s.parse::<u64>().unwrap_or(0))
                .collect()
        })
        .unwrap_or_default()
    };
    let mut left = parse_line(lines.next());
    let mut right = parse_line(lines.next());
    left.resize(backward_size, 0);
    right.resize(forward_size, 0);
    Ok((left, right))
}

/// Runtime instrumentation that tallies connection-matrix accesses per context ID.
///
/// Enabled by the `ctxfreq` feature only; normal builds compile none of this and the
/// `cost()` hot path is untouched. Used to derive a corpus-frequency remap: run a
/// tokenization over a corpus, then [`dump_ctx_freq`] the histograms and point
/// `LINDERA_CTX_FREQ_FILE` at them when building the dictionary.
#[cfg(feature = "ctxfreq")]
mod ctxfreq {
    use std::cell::RefCell;
    use std::path::Path;

    thread_local! {
        /// Per-left-context-id (matrix backward axis) access counts.
        static HIST_LEFT: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
        /// Per-right-context-id (matrix forward axis) access counts.
        static HIST_RIGHT: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    }

    /// Increment `hist[idx]`, growing the histogram as needed.
    fn bump(hist: &RefCell<Vec<u64>>, idx: usize) {
        let mut v = hist.borrow_mut();
        if idx >= v.len() {
            v.resize(idx + 1, 0);
        }
        v[idx] += 1;
    }

    /// Record one connection-matrix access.
    ///
    /// # Arguments
    ///
    /// * `forward_id` - Right-context id of the left node (matrix forward axis).
    /// * `backward_id` - Left-context id of the right node (matrix backward axis).
    pub fn record_access(forward_id: u32, backward_id: u32) {
        HIST_RIGHT.with(|h| bump(h, forward_id as usize));
        HIST_LEFT.with(|h| bump(h, backward_id as usize));
    }

    /// Write the accumulated histograms to `path`, padded to the given axis sizes.
    ///
    /// # Arguments
    ///
    /// * `path` - Destination file.
    /// * `forward_size` - Right-context axis size to pad/truncate to.
    /// * `backward_size` - Left-context axis size to pad/truncate to.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the underlying I/O error.
    pub fn dump(path: &Path, forward_size: usize, backward_size: usize) -> std::io::Result<()> {
        use std::io::Write;

        let take = |h: &'static std::thread::LocalKey<RefCell<Vec<u64>>>, n: usize| -> Vec<u64> {
            h.with(|c| {
                let mut v = c.borrow().clone();
                v.resize(n, 0);
                v
            })
        };
        let left = take(&HIST_LEFT, backward_size);
        let right = take(&HIST_RIGHT, forward_size);

        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        writeln!(f, "{backward_size} {forward_size}")?;
        let write_row =
            |f: &mut std::io::BufWriter<std::fs::File>, row: &[u64]| -> std::io::Result<()> {
                let mut first = true;
                for c in row {
                    if !first {
                        write!(f, " ")?;
                    }
                    write!(f, "{c}")?;
                    first = false;
                }
                writeln!(f)
            };
        write_row(&mut f, &left)?;
        write_row(&mut f, &right)?;
        f.flush()
    }
}

#[cfg(feature = "ctxfreq")]
pub use ctxfreq::{dump as dump_ctx_freq, record_access};

#[cfg(test)]
mod tests {
    use super::*;

    /// `build_perm` pins 0, ranks by count desc then id asc, and stays a bijection.
    #[test]
    fn test_build_perm_orders_by_frequency() {
        // id:   0    1    2    3    4
        // count 99   5    50   50   0
        let hist = [99u64, 5, 50, 50, 0];
        let perm = build_perm(&hist);
        assert_eq!(perm[0], 0); // BOS/EOS pinned
        // ids 2 and 3 tie on count(50); id asc breaks the tie -> 2 before 3.
        assert_eq!(perm[2], 1);
        assert_eq!(perm[3], 2);
        assert_eq!(perm[1], 3); // count 5
        assert_eq!(perm[4], 4); // count 0 (matrix-only) last
        // Bijection: every new id 0..n appears exactly once.
        let mut seen = perm.clone();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2, 3, 4]);
    }

    /// An empty histogram yields an empty permutation (no panic).
    #[test]
    fn test_build_perm_empty() {
        assert!(build_perm(&[]).is_empty());
    }

    /// A single-element histogram maps 0 -> 0.
    #[test]
    fn test_build_perm_single() {
        assert_eq!(build_perm(&[7]), vec![0]);
    }
}

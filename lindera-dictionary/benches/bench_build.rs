//! Criterion benchmarks for the dictionary build pipeline.
//!
//! These benchmarks measure [`DictionaryBuilder`] against a real dictionary
//! source directory. Dictionary sources are large and not bundled in the
//! repository, so the source location is supplied via environment variables:
//!
//! - `LINDERA_BENCH_DICTIONARY_SOURCE_DIR` (required): directory containing
//!   the lexicon CSV files, `matrix.def`, `char.def`, and `unk.def`
//!   (e.g. a mecab-ipadic checkout converted to UTF-8).
//! - `LINDERA_BENCH_DICTIONARY_METADATA` (optional): path to a
//!   `metadata.json` matching the source. Defaults to the IPADIC metadata
//!   bundled in this workspace (`lindera-ipadic/metadata.json`).
//!
//! When `LINDERA_BENCH_DICTIONARY_SOURCE_DIR` is not set, all benchmarks are
//! skipped so that `cargo bench` remains runnable without dictionary data
//! (e.g. in CI).
//!
//! Typical before/after comparison:
//!
//! ```sh
//! # On the baseline branch:
//! LINDERA_BENCH_DICTIONARY_SOURCE_DIR=/path/to/mecab-ipadic \
//!   cargo bench -p lindera-dictionary --bench bench_build -- --save-baseline before
//!
//! # After applying optimizations:
//! LINDERA_BENCH_DICTIONARY_SOURCE_DIR=/path/to/mecab-ipadic \
//!   cargo bench -p lindera-dictionary --bench bench_build -- --baseline before
//! ```

use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::dictionary::metadata::Metadata;

/// Returns the dictionary source directory from the environment, or `None`
/// (with a notice on stderr) when the benchmarks should be skipped.
fn source_dir() -> Option<PathBuf> {
    match std::env::var_os("LINDERA_BENCH_DICTIONARY_SOURCE_DIR") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => {
            eprintln!(
                "LINDERA_BENCH_DICTIONARY_SOURCE_DIR is not set; skipping dictionary build benchmarks."
            );
            None
        }
    }
}

/// Loads the dictionary metadata from `LINDERA_BENCH_DICTIONARY_METADATA`,
/// falling back to the IPADIC metadata bundled in this workspace.
fn load_metadata() -> Metadata {
    let metadata_path = std::env::var_os("LINDERA_BENCH_DICTIONARY_METADATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("lindera-ipadic")
                .join("metadata.json")
        });

    let metadata_file = File::open(&metadata_path)
        .unwrap_or_else(|err| panic!("failed to open metadata {metadata_path:?}: {err}"));
    serde_json::from_reader(metadata_file)
        .unwrap_or_else(|err| panic!("failed to parse metadata {metadata_path:?}: {err}"))
}

/// Measures the full `DictionaryBuilder::build_dictionary` pipeline
/// (metadata, character definition, unknown dictionary, prefix dictionary,
/// and connection cost matrix), including output file writes.
fn bench_build_dictionary(c: &mut Criterion) {
    let Some(input_dir) = source_dir() else {
        return;
    };
    let builder = DictionaryBuilder::new(load_metadata());
    let output_dir = tempfile::tempdir().unwrap();

    let mut group = c.benchmark_group("build");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(60));
    group.bench_function("bench-build-dictionary", |b| {
        b.iter(|| {
            builder
                .build_dictionary(&input_dir, output_dir.path())
                .unwrap();
        })
    });
    group.finish();
}

/// Measures the prefix dictionary stage alone (lexicon CSV parse, sort,
/// word entry grouping, double array construction, and serialization).
fn bench_build_prefix_dictionary(c: &mut Criterion) {
    let Some(input_dir) = source_dir() else {
        return;
    };
    let builder = DictionaryBuilder::new(load_metadata());
    let output_dir = tempfile::tempdir().unwrap();

    let mut group = c.benchmark_group("build-stages");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(60));
    group.bench_function("bench-build-prefix-dictionary", |b| {
        b.iter(|| {
            builder
                .build_prefix_dictionary(&input_dir, output_dir.path())
                .unwrap();
        })
    });
    group.finish();
}

/// Measures the connection cost matrix stage alone (`matrix.def` parse and
/// `matrix.mtx` serialization).
fn bench_build_connection_cost_matrix(c: &mut Criterion) {
    let Some(input_dir) = source_dir() else {
        return;
    };
    let builder = DictionaryBuilder::new(load_metadata());
    let output_dir = tempfile::tempdir().unwrap();

    let mut group = c.benchmark_group("build-stages");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(30));
    group.bench_function("bench-build-connection-cost-matrix", |b| {
        b.iter(|| {
            builder
                .build_connection_cost_matrix(&input_dir, output_dir.path())
                .unwrap();
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_build_dictionary,
    bench_build_prefix_dictionary,
    bench_build_connection_cost_matrix
);
criterion_main!(benches);

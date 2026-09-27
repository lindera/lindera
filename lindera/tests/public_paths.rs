//! Compile-time check of the public paths of the `lindera` facade.
//!
//! Every `use ... as _;` below must resolve, so a path that disappears from
//! the facade fails `cargo test -p lindera` at compile time. The generated
//! part lists every module-level item of the `lindera` crate as published in
//! 6.2.0, taken from `ci/public-api/lindera-6.2.0-default.txt` and
//! `lindera-6.2.0-train.txt`; see "Regenerating `lindera/tests/public_paths.rs`"
//! in `ci/public-api/README.md` for the procedure. The hand-written sections
//! at the end cover the paths that only exist since the facade.
//!
//! `use path as _;` binds nothing, so the file also compiles with
//! `--no-default-features`. The single test function exists so that the file
//! is always run.
#![allow(unused_imports)]

// ---- Generated from ci/public-api/lindera-6.2.0-default.txt ----

use lindera::LinderaResult as _;
use lindera::dictionary as _;
use lindera::error as _;
use lindera::get_version as _;
use lindera::mode as _;
use lindera::segmenter as _;
use lindera::space_penalty as _;
use lindera::token as _;
use lindera::worker as _;

use lindera::dictionary::Dictionary as _;
use lindera::dictionary::DictionaryBuilder as _;
use lindera::dictionary::DictionaryConfig as _;
use lindera::dictionary::DictionaryKind as _;
use lindera::dictionary::DictionaryKindIter as _;
use lindera::dictionary::DictionaryScheme as _;
use lindera::dictionary::DictionarySchemeIter as _;
use lindera::dictionary::FieldDefinition as _;
use lindera::dictionary::FieldType as _;
use lindera::dictionary::Lattice as _;
use lindera::dictionary::Metadata as _;
use lindera::dictionary::Schema as _;
use lindera::dictionary::UserDictionary as _;
use lindera::dictionary::UserDictionaryConfig as _;
use lindera::dictionary::WordId as _;
use lindera::dictionary::load_dictionary as _;
use lindera::dictionary::load_dictionary_with_options as _;
use lindera::dictionary::load_embedded_dictionary as _;
use lindera::dictionary::load_fs_dictionary as _;
use lindera::dictionary::load_fs_dictionary_with_options as _;
use lindera::dictionary::load_user_dictionary as _;
use lindera::dictionary::load_user_dictionary_from_bin as _;
use lindera::dictionary::load_user_dictionary_from_csv as _;
use lindera::dictionary::resolve_embedded_loader as _;

use lindera::error::LinderaError as _;
use lindera::error::LinderaErrorKind as _;

use lindera::mode::Mode as _;
use lindera::mode::Penalty as _;

use lindera::segmenter::Segmenter as _;
use lindera::segmenter::SegmenterConfig as _;

use lindera::space_penalty::SpacePenaltyConfig as _;
use lindera::space_penalty::SpacePenaltyRule as _;
use lindera::space_penalty::SpacePenaltyTable as _;

use lindera::token::Token as _;

use lindera::worker::SegmentWorker as _;

// ---- Generated from ci/public-api/lindera-6.2.0-train.txt (train only) ----

#[cfg(feature = "train")]
use lindera::dictionary::trainer as _;

// ---- Hand-written: paths that exist since the facade ----

// The text analysis chain, re-exported as `lindera::analysis` with the
// `analysis` feature.
#[cfg(feature = "analysis")]
use lindera::analysis::character_filter::CharacterFilterLoader;
#[cfg(feature = "analysis")]
use lindera::analysis::token_filter::TokenFilterLoader;
#[cfg(feature = "analysis")]
use lindera::analysis::tokenizer::{Tokenizer, TokenizerBuilder};
#[cfg(feature = "analysis")]
use lindera::analysis::{character_filter, token_filter, tokenizer, worker};

// The raw `lindera-dictionary` API, re-exported under `lindera::dictionary`.
use lindera::dictionary::builder::DictionaryBuilder;
use lindera::dictionary::core::character_definition::CharacterDefinition;
use lindera::dictionary::core::connection_cost_matrix::ConnectionCostMatrix;
use lindera::dictionary::core::prefix_dictionary::PrefixDictionary;
use lindera::dictionary::core::unknown_dictionary::UnknownDictionary;
use lindera::dictionary::core::{Dictionary, UserDictionary};
use lindera::dictionary::viterbi::Lattice;
use lindera::dictionary::{embedded_dictionary, include_bytes_aligned};

#[test]
fn facade_reports_a_version() {
    assert!(!lindera::get_version().is_empty());
}

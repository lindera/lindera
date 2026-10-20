//! User dictionary building shared by the bindings.
//!
//! The bindings' standalone user dictionary builders take metadata only, so
//! a CSV row whose context ID lies outside the system dictionary's
//! connection matrix is reported only when the result is combined with the
//! system dictionary, by word id. [`build_user_dictionary_for`] builds for a
//! loaded system dictionary instead, so such a row is reported by its CSV
//! line and surface as the CSV is built, as `lindera build --user
//! --system-dict` does.

use std::path::Path;

use lindera::LinderaResult;
use lindera::dictionary::{Dictionary, DictionaryBuilder};

/// Builds a user dictionary from a CSV file for a system dictionary and
/// writes it to a file.
///
/// The CSV is built with the system dictionary's metadata, as
/// `lindera::dictionary::load_user_dictionary_for` builds one, and every
/// row's context IDs are checked against its connection matrix
/// (`DictionaryBuilder::with_system_matrix`). A row outside the matrix fails
/// the build with an error that names its CSV line and surface, and no file
/// is written.
///
/// # Arguments
///
/// * `input_file` - Path of the CSV file with the user dictionary entries.
/// * `output_file` - Path of the file to write the built user dictionary to.
///   Its parent directory is created if needed. The user dictionary loaders
///   choose how to load a file by its extension, so end it with `.bin`.
/// * `dictionary` - The system dictionary the user dictionary is built for.
///
/// # Returns
///
/// `Ok(())` once the file is written, or an error if the CSV cannot be read
/// or built, a row has a context ID outside the system dictionary's
/// connection matrix, or the file cannot be written.
pub fn build_user_dictionary_for(
    input_file: &Path,
    output_file: &Path,
    dictionary: &Dictionary,
) -> LinderaResult<()> {
    DictionaryBuilder::new((*dictionary.metadata).clone())
        .with_system_matrix(&dictionary.connection_cost_matrix)
        .build_user_dictionary(input_file, output_file)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "embed-ipadic")]
    mod with_ipadic {
        use std::path::{Path, PathBuf};

        use lindera::dictionary::{Dictionary, load_dictionary, load_user_dictionary_for};
        use lindera::mode::Mode;

        use super::super::build_user_dictionary_for;
        use crate::tokenizer::CoreTokenizer;

        // Line 1 lies inside IPADIC's 1316 x 1316 connection matrix; line 2
        // has the context IDs 60000, far outside it.
        const OUT_OF_RANGE_CSV: &str = "\
東京タワー,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,東京タワー,トウキョウタワー,トウキョウタワー
東京スカイツリー,60000,60000,-1000,名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー
";

        const SIMPLE_CSV: &str = "東京スカイツリー,カスタム名詞,トウキョウスカイツリー\n";

        fn ipadic() -> Dictionary {
            match load_dictionary("embedded://ipadic") {
                Ok(dictionary) => dictionary,
                Err(err) => panic!("failed to load embedded IPADIC: {err}"),
            }
        }

        fn write_csv(dir: &Path, content: &str) -> PathBuf {
            let csv = dir.join("userdic.csv");
            if let Err(err) = std::fs::write(&csv, content) {
                panic!("failed to write {csv:?}: {err}");
            }
            csv
        }

        fn surfaces(tokenizer: &CoreTokenizer, text: &str) -> Vec<String> {
            match tokenizer.tokenize(text) {
                Ok(tokens) => tokens.into_iter().map(|token| token.surface).collect(),
                Err(err) => panic!("failed to tokenize {text:?}: {err}"),
            }
        }

        #[test]
        fn build_user_dictionary_for_names_the_csv_line_and_surface() {
            let dir = match tempfile::tempdir() {
                Ok(dir) => dir,
                Err(err) => panic!("failed to create a temporary directory: {err}"),
            };
            let csv = write_csv(dir.path(), OUT_OF_RANGE_CSV);
            let output = dir.path().join("userdic.bin");

            let err = match build_user_dictionary_for(&csv, &output, &ipadic()) {
                Ok(()) => panic!("built a user dictionary with a row outside the matrix"),
                Err(err) => format!("{err:?}"),
            };
            assert!(
                err.contains("line 2 (surface: '東京スカイツリー')"),
                "unexpected error: {err}"
            );
            assert!(
                err.contains("left context id 60000 is out of range"),
                "unexpected error: {err}"
            );
            assert!(!output.exists(), "{output:?} must not be written");
        }

        #[test]
        fn build_user_dictionary_for_writes_the_output_file() {
            let dir = match tempfile::tempdir() {
                Ok(dir) => dir,
                Err(err) => panic!("failed to create a temporary directory: {err}"),
            };
            let csv = write_csv(dir.path(), SIMPLE_CSV);
            let output = dir.path().join("out").join("userdic.bin");
            let dictionary = ipadic();

            let baseline =
                match CoreTokenizer::from_segmenter(Mode::Normal, dictionary.clone(), None) {
                    Ok(tokenizer) => surfaces(&tokenizer, "東京スカイツリー"),
                    Err(err) => panic!("failed to build the tokenizer: {err}"),
                };
            assert!(baseline.len() > 1, "fixture assumption: {baseline:?}");

            if let Err(err) = build_user_dictionary_for(&csv, &output, &dictionary) {
                panic!("failed to build the user dictionary: {err}");
            }
            assert!(output.is_file(), "{output:?} must be a file");

            let output_uri = match output.to_str() {
                Some(uri) => uri,
                None => panic!("{output:?} is not UTF-8"),
            };
            let user_dictionary = match load_user_dictionary_for(output_uri, &dictionary) {
                Ok(user_dictionary) => user_dictionary,
                Err(err) => panic!("failed to load {output:?}: {err}"),
            };
            let tokenizer = match CoreTokenizer::from_segmenter(
                Mode::Normal,
                dictionary,
                Some(user_dictionary),
            ) {
                Ok(tokenizer) => tokenizer,
                Err(err) => panic!("failed to build the tokenizer: {err}"),
            };
            assert_eq!(
                surfaces(&tokenizer, "東京スカイツリー"),
                ["東京スカイツリー"]
            );
        }
    }
}

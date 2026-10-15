pub mod character_definition;
pub mod connection_cost_matrix;
pub mod context_id_remap;
pub mod metadata;
pub mod prefix_dictionary;
pub mod unknown_dictionary;
pub mod user_dictionary;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use csv::StringRecord;

use self::character_definition::CharacterDefinitionBuilderOptions;
use self::connection_cost_matrix::{
    ConnectionCostMatrixBuilderOptions, MatrixSize, read_matrix_header,
};
use self::context_id_remap::compute_context_id_remap_for_matrix;
use self::metadata::MetadataBuilder;
use self::prefix_dictionary::{PrefixDictionaryBuilder, PrefixDictionaryBuilderOptions};
use self::unknown_dictionary::{UnknownDictionaryBuilder, UnknownDictionaryBuilderOptions};
use self::user_dictionary::{
    UserDictionaryBuilder, UserDictionaryBuilderOptions, build_user_dictionary,
};
use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::metadata::{DICTIONARY_FORMAT_VERSION, Metadata};
use crate::error::LinderaErrorKind;

#[derive(Clone)]
pub struct DictionaryBuilder {
    metadata: Metadata,
    /// Optional path to a bundled context-ID access-frequency histogram, used to
    /// rank context IDs when `metadata.connection_id_mapping` is enabled. Shipped
    /// alongside `metadata.json` in the dictionary crate; see
    /// [`context_id_remap::compute_context_id_remap`] for the precedence rules.
    context_id_freq: Option<std::path::PathBuf>,
    /// The connection matrix size of the system dictionary a user dictionary
    /// is built for, set by [`Self::with_system_matrix`]. `None` builds a
    /// user dictionary without checking its context IDs.
    system_matrix_size: Option<MatrixSize>,
}

impl DictionaryBuilder {
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            context_id_freq: None,
            system_matrix_size: None,
        }
    }

    /// Returns the source metadata this builder was created with.
    ///
    /// # Returns
    ///
    /// The metadata read from the dictionary crate's `metadata.json`.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Attach a bundled context-ID frequency histogram used for the connection-cost
    /// remap ranking.
    ///
    /// # Arguments
    ///
    /// * `path` - Histogram file (as produced by the `ctxfreq` instrumentation).
    ///
    /// # Returns
    ///
    /// The builder with the frequency source attached.
    pub fn with_context_id_freq(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.context_id_freq = Some(path.into());
        self
    }

    /// Returns the bundled context-ID frequency histogram attached with
    /// [`Self::with_context_id_freq`].
    ///
    /// # Returns
    ///
    /// The histogram's path, or `None` when none is attached. The build may
    /// still read another file: see
    /// [`context_id_remap::resolve_context_id_freq`].
    #[cfg(feature = "build_rs")]
    pub(crate) fn context_id_freq(&self) -> Option<&Path> {
        self.context_id_freq.as_deref()
    }

    /// Names the connection matrix of the system dictionary a user
    /// dictionary is built for, so that the build checks every row's context
    /// IDs against it, as MeCab's `mecab-dict-index -u` does.
    ///
    /// The bounds are those [`Self::build_dictionary`] applies to the system
    /// dictionary's own lexicon: a left context ID must be below the matrix's
    /// backward size and a right context ID below its forward size. A row
    /// outside them fails [`Self::build_user_dictionary`],
    /// [`Self::build_user_dict`] and [`Self::build_user_dict_from_reader`]
    /// with an error that names its CSV line and surface. Without this, a
    /// user dictionary is checked only when it is combined with the system
    /// dictionary ([`UserDictionary::check_context_ids`]), whose error names
    /// the entry by word id. A system dictionary build ignores the matrix,
    /// since it checks against its own `matrix.def`.
    ///
    /// Only the matrix's size is kept.
    ///
    /// # Arguments
    ///
    /// * `matrix` - The connection matrix of the system dictionary, such as
    ///   `Dictionary::connection_cost_matrix` of a loaded dictionary.
    ///
    /// # Returns
    ///
    /// The builder with the system matrix size set.
    pub fn with_system_matrix(mut self, matrix: &ConnectionCostMatrix) -> Self {
        self.system_matrix_size = Some(MatrixSize::from_matrix(matrix));
        self
    }

    /// Build all dictionary artifacts from `input_dir` into `output_dir`.
    ///
    /// The independent stages run concurrently on non-wasm targets and
    /// sequentially on wasm (which has no OS threads). The output files are
    /// identical regardless of the path taken.
    ///
    /// The context ids of every lexicon and `unk.def` entry, and the ids of
    /// every `matrix.def` entry, are checked against the axis sizes of the
    /// `matrix.def` header, as MeCab's `mecab-dict-index` does: an id out of
    /// range fails the build (a lexicon row is skipped with a warning instead
    /// when `skip_invalid_cost_or_id` is set), rather than building a
    /// dictionary that indexes past its connection matrix at tokenize time.
    /// The stage methods run on their own do not read the header, so they
    /// check only the `matrix.def` entries.
    ///
    /// `char.def` and `unk.def` are checked as `mecab-dict-index` checks
    /// them: a range line must name only categories defined on earlier
    /// lines, a category must be defined once, every `unk.def` line must name
    /// a `char.def` category, and every `char.def` category must have an
    /// `unk.def` line. The stage methods run on their own check them too.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the source dictionary files.
    /// * `output_dir` - Directory to write the built artifacts into.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the first stage error in stage order.
    pub fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        // Read the matrix.def header ONCE, serially, before the independent stages
        // start: the prefix and unknown stages check their entries' context ids
        // against it, and the remap is sized by it.
        let matrix_size = read_matrix_header(input_dir, &self.metadata.encoding)?;

        // Compute the connection-cost context-ID remap ONCE, serially, before the
        // independent stages start, so the prefix / unknown / matrix stages all apply
        // the same permutations. `None` (flag off) keeps every stage byte-identical.
        let remap: Option<Arc<ContextIdMap>> = if self.metadata.connection_id_mapping {
            Some(Arc::new(compute_context_id_remap_for_matrix(
                input_dir,
                &self.metadata,
                self.context_id_freq.as_deref(),
                matrix_size,
            )?))
        } else {
            None
        };

        #[cfg(not(target_family = "wasm"))]
        {
            self.build_dictionary_parallel(input_dir, output_dir, remap.as_ref(), matrix_size)
        }
        #[cfg(target_family = "wasm")]
        {
            self.build_dictionary_sequential(input_dir, output_dir, remap.as_ref(), matrix_size)
        }
    }

    /// Build every stage sequentially.
    ///
    /// Used on wasm targets, and as the reference ordering: metadata,
    /// character definition, unknown dictionary, prefix dictionary, then
    /// connection cost matrix.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the source dictionary files.
    /// * `output_dir` - Directory to write the built artifacts into.
    /// * `remap` - The context-id remap every stage applies, if any.
    /// * `matrix_size` - The `matrix.def` axis sizes the entries' context ids
    ///   are checked against.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the first stage error.
    #[cfg(target_family = "wasm")]
    fn build_dictionary_sequential(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<&Arc<ContextIdMap>>,
        matrix_size: MatrixSize,
    ) -> LinderaResult<()> {
        self.build_metadata(output_dir, remap)?;
        let chardef = self.build_character_definition(input_dir, output_dir)?;
        self.unknown_dictionary_builder(remap.cloned(), Some(matrix_size))
            .build(input_dir, &chardef, output_dir)?;
        self.prefix_dictionary_builder(remap.cloned(), Some(matrix_size))
            .build(input_dir, output_dir)?;
        self.build_connection_cost_matrix(input_dir, output_dir, remap.cloned())?;

        Ok(())
    }

    /// Build the four independent stage chains concurrently.
    ///
    /// The only data dependency is `character definition -> unknown
    /// dictionary`; the metadata, prefix dictionary, and connection cost matrix
    /// stages are independent and write disjoint files, so each chain runs on
    /// its own scoped thread. All threads are joined before results are
    /// inspected, and the earliest failure in stage order is returned so the
    /// result matches the sequential fail-fast order; a panicked stage is
    /// re-raised rather than swallowed.
    ///
    /// Peak memory is higher than the sequential path, since the working sets
    /// of the concurrent stages (most notably the prefix dictionary and
    /// connection cost matrix) are held at the same time.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the source dictionary files.
    /// * `output_dir` - Directory to write the built artifacts into.
    /// * `remap` - The context-id remap every stage applies, if any.
    /// * `matrix_size` - The `matrix.def` axis sizes the entries' context ids
    ///   are checked against.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or the earliest stage error in stage order.
    #[cfg(not(target_family = "wasm"))]
    fn build_dictionary_parallel(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<&Arc<ContextIdMap>>,
        matrix_size: MatrixSize,
    ) -> LinderaResult<()> {
        std::thread::scope(|scope| {
            let metadata = scope.spawn(move || self.build_metadata(output_dir, remap));
            let unknown = scope.spawn(move || {
                let chardef = self.build_character_definition(input_dir, output_dir)?;
                self.unknown_dictionary_builder(remap.cloned(), Some(matrix_size))
                    .build(input_dir, &chardef, output_dir)
            });
            let prefix = scope.spawn(move || {
                self.prefix_dictionary_builder(remap.cloned(), Some(matrix_size))
                    .build(input_dir, output_dir)
            });
            let matrix = scope.spawn(move || {
                self.build_connection_cost_matrix(input_dir, output_dir, remap.cloned())
            });

            // Join all stages, then report the earliest failure in stage order.
            let results = [
                metadata.join(),
                unknown.join(),
                prefix.join(),
                matrix.join(),
            ];
            for result in results {
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => return Err(err),
                    Err(panic) => std::panic::resume_unwind(panic),
                }
            }

            Ok(())
        })
    }

    /// Write `metadata.json`, stamping the dictionary format version and
    /// embedding the context-ID permutation when one was applied.
    ///
    /// The format version is taken from
    /// [`DICTIONARY_FORMAT_VERSION`] rather than from the source metadata:
    /// it describes the artifacts this builder just wrote, so the builder is
    /// the only thing that can state it truthfully. A source `metadata.json`
    /// carrying a stale (or invented) version must not be able to mislabel a
    /// freshly built dictionary.
    ///
    /// Persisting the permutation is what lets a user dictionary compiled later be
    /// relabeled into the same ID space (see
    /// [`crate::dictionary::UserDictionary::remap_context_ids`]).
    ///
    /// # Arguments
    ///
    /// * `output_dir` - Directory to write `metadata.json` into.
    /// * `remap` - The permutation applied by this build, if any.
    pub fn build_metadata(
        &self,
        output_dir: &Path,
        remap: Option<&Arc<ContextIdMap>>,
    ) -> LinderaResult<()> {
        let mut metadata = self.metadata.clone();
        metadata.format_version = DICTIONARY_FORMAT_VERSION;
        if let Some(map) = remap {
            metadata.context_id_map = Some(ContextIdMap::clone(map));
        }
        MetadataBuilder::new().build(&metadata, output_dir)
    }

    pub fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        CharacterDefinitionBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .builder()
            .build(input_dir, output_dir)
    }

    /// Builds the unknown-word dictionary (`unk.bin`) from `unk.def` alone.
    ///
    /// Run on its own, the stage does not read `matrix.def`, so it does not
    /// check the entries' context ids against the connection matrix;
    /// [`Self::build_dictionary`] does. It does check the entries'
    /// categories against those of `chardef`.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing `unk.def`.
    /// * `output_dir` - Directory to write `unk.bin` into.
    /// * `chardef` - The character definition the categories come from.
    /// * `remap` - The context-id remap to relabel the entries with, if any.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if `unk.def` cannot be read or
    /// parsed, a line names a category `chardef` does not define, or a
    /// category of `chardef` has no line.
    pub fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
        remap: Option<Arc<ContextIdMap>>,
    ) -> LinderaResult<()> {
        self.unknown_dictionary_builder(remap, None)
            .build(input_dir, chardef, output_dir)
    }

    /// Configures the unknown-dictionary stage from this builder's metadata.
    ///
    /// # Arguments
    ///
    /// * `remap` - The context-id remap to relabel the entries with, if any.
    /// * `matrix_size` - The connection matrix size to check the entries'
    ///   context ids against, or `None` to skip the check.
    ///
    /// # Returns
    ///
    /// The configured stage builder.
    fn unknown_dictionary_builder(
        &self,
        remap: Option<Arc<ContextIdMap>>,
        matrix_size: Option<MatrixSize>,
    ) -> UnknownDictionaryBuilder {
        UnknownDictionaryBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .context_id_remap(remap)
            .matrix_size(matrix_size)
            .builder()
    }

    /// Builds the prefix dictionary (`dict.*`) from the lexicon CSV files
    /// alone.
    ///
    /// Run on its own, the stage does not read `matrix.def`, so it does not
    /// check the rows' context ids against the connection matrix;
    /// [`Self::build_dictionary`] does.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the lexicon `*.csv` files.
    /// * `output_dir` - Directory to write the `dict.*` files into.
    /// * `remap` - The context-id remap to relabel the rows with, if any.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if a file cannot be read or a row is
    /// invalid.
    pub fn build_prefix_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<Arc<ContextIdMap>>,
    ) -> LinderaResult<()> {
        self.prefix_dictionary_builder(remap, None)
            .build(input_dir, output_dir)
    }

    /// Configures the prefix-dictionary stage from this builder's metadata.
    ///
    /// # Arguments
    ///
    /// * `remap` - The context-id remap to relabel the rows with, if any.
    /// * `matrix_size` - The connection matrix size to check the rows' context
    ///   ids against, or `None` to skip the check.
    ///
    /// # Returns
    ///
    /// The configured stage builder.
    fn prefix_dictionary_builder(
        &self,
        remap: Option<Arc<ContextIdMap>>,
        matrix_size: Option<MatrixSize>,
    ) -> PrefixDictionaryBuilder {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(self.metadata.flexible_csv)
            .encoding(self.metadata.encoding.clone())
            .skip_invalid_cost_or_id(self.metadata.skip_invalid_cost_or_id)
            .schema(self.metadata.dictionary_schema.clone())
            .context_id_remap(remap)
            .matrix_size(matrix_size)
            .builder()
    }

    pub fn build_connection_cost_matrix(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<Arc<ContextIdMap>>,
    ) -> LinderaResult<()> {
        ConnectionCostMatrixBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .context_id_remap(remap)
            .builder()
            .build(input_dir, output_dir)
    }

    pub fn build_user_dictionary(
        &self,
        input_file: &Path,
        output_file: &Path,
    ) -> LinderaResult<()> {
        let user_dict = self.build_user_dict(input_file)?;
        build_user_dictionary(user_dict, output_file)
    }

    pub fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        self.user_dictionary_builder().build(input_file)
    }

    /// Builds a user dictionary from CSV content supplied by a reader.
    ///
    /// The filesystem-free counterpart of [`Self::build_user_dict`], added so
    /// WebAssembly callers can build a user dictionary from bytes (#972).
    /// The content must be UTF-8.
    ///
    /// # Arguments
    ///
    /// * `reader` - Read source of the user dictionary CSV content.
    ///
    /// # Returns
    ///
    /// The built user dictionary.
    pub fn build_user_dict_from_reader<R: std::io::Read>(
        &self,
        reader: R,
    ) -> LinderaResult<UserDictionary> {
        self.user_dictionary_builder().build_from_reader(reader)
    }

    /// Composes the user-dictionary builder from this builder's metadata,
    /// including the handler that maps simple rows onto the dictionary
    /// schema's custom fields.
    ///
    /// # Returns
    ///
    /// The configured builder, shared by the path and reader entry points.
    fn user_dictionary_builder(&self) -> UserDictionaryBuilder {
        let userdic_schema = self.metadata.user_dictionary_schema.clone();
        let dict_schema = self.metadata.dictionary_schema.clone();
        let default_field_value = self.metadata.default_field_value.clone();

        UserDictionaryBuilderOptions::default()
            .user_dictionary_fields_num(self.metadata.user_dictionary_schema.field_count())
            .dictionary_fields_num(self.metadata.dictionary_schema.field_count())
            .default_word_cost(self.metadata.default_word_cost)
            .default_left_context_id(self.metadata.default_left_context_id)
            .default_right_context_id(self.metadata.default_right_context_id)
            .flexible_csv(self.metadata.flexible_csv)
            .matrix_size(self.system_matrix_size)
            .user_dictionary_handler(Some(Box::new(move |row: &StringRecord| {
                // Map user dictionary fields to dictionary schema fields
                let mut result = Vec::new();

                // Skip the first 4 common fields (surface, left_id, right_id, cost)
                for field_name in dict_schema.get_custom_fields() {
                    if let Some(idx) = userdic_schema.get_field_index(field_name) {
                        // If field exists in user dictionary schema, get value from CSV
                        if idx < row.len() {
                            result.push(row[idx].to_string());
                        } else {
                            result.push(default_field_value.clone());
                        }
                    } else {
                        // Field not in user dictionary schema, use default value
                        result.push(default_field_value.clone());
                    }
                }

                Ok(result)
            })))
            .builder()
    }
}

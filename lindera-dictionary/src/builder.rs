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
use self::connection_cost_matrix::ConnectionCostMatrixBuilderOptions;
use self::context_id_remap::{ContextIdRemap, compute_context_id_remap};
use self::metadata::MetadataBuilder;
use self::prefix_dictionary::PrefixDictionaryBuilderOptions;
use self::unknown_dictionary::UnknownDictionaryBuilderOptions;
use self::user_dictionary::{UserDictionaryBuilderOptions, build_user_dictionary};
use crate::LinderaResult;
use crate::dictionary::UserDictionary;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::metadata::Metadata;
use crate::error::LinderaErrorKind;

#[derive(Clone)]
pub struct DictionaryBuilder {
    metadata: Metadata,
    /// Optional path to a bundled context-ID access-frequency histogram, used to
    /// rank context IDs when `metadata.connection_id_mapping` is enabled. Shipped
    /// alongside `metadata.json` in the dictionary crate; see
    /// [`context_id_remap::compute_context_id_remap`] for the precedence rules.
    context_id_freq: Option<std::path::PathBuf>,
}

impl DictionaryBuilder {
    pub fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            context_id_freq: None,
        }
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

    /// Build all dictionary artifacts from `input_dir` into `output_dir`.
    ///
    /// The independent stages run concurrently on non-wasm targets and
    /// sequentially on wasm (which has no OS threads). The output files are
    /// identical regardless of the path taken.
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

        // Compute the connection-cost context-ID remap ONCE, serially, before the
        // independent stages start, so the prefix / unknown / matrix stages all apply
        // the same permutations. `None` (flag off) keeps every stage byte-identical.
        let remap: Option<Arc<ContextIdRemap>> = if self.metadata.connection_id_mapping {
            Some(Arc::new(compute_context_id_remap(
                input_dir,
                &self.metadata,
                self.context_id_freq.as_deref(),
            )?))
        } else {
            None
        };

        #[cfg(not(target_family = "wasm"))]
        {
            self.build_dictionary_parallel(input_dir, output_dir, remap.as_ref())
        }
        #[cfg(target_family = "wasm")]
        {
            self.build_dictionary_sequential(input_dir, output_dir, remap.as_ref())
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
    #[cfg(target_family = "wasm")]
    fn build_dictionary_sequential(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<&Arc<ContextIdRemap>>,
    ) -> LinderaResult<()> {
        self.build_metadata(output_dir)?;
        let chardef = self.build_character_definition(input_dir, output_dir)?;
        self.build_unknown_dictionary(input_dir, output_dir, &chardef, remap.cloned())?;
        self.build_prefix_dictionary(input_dir, output_dir, remap.cloned())?;
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
    #[cfg(not(target_family = "wasm"))]
    fn build_dictionary_parallel(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<&Arc<ContextIdRemap>>,
    ) -> LinderaResult<()> {
        std::thread::scope(|scope| {
            let metadata = scope.spawn(move || self.build_metadata(output_dir));
            let unknown = scope.spawn(move || {
                let chardef = self.build_character_definition(input_dir, output_dir)?;
                self.build_unknown_dictionary(input_dir, output_dir, &chardef, remap.cloned())
            });
            let prefix = scope
                .spawn(move || self.build_prefix_dictionary(input_dir, output_dir, remap.cloned()));
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

    pub fn build_metadata(&self, output_dir: &Path) -> LinderaResult<()> {
        MetadataBuilder::new().build(&self.metadata, output_dir)
    }

    pub fn build_character_definition(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinition> {
        CharacterDefinitionBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .builder()
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?
            .build(input_dir, output_dir)
    }

    pub fn build_unknown_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        chardef: &CharacterDefinition,
        remap: Option<Arc<ContextIdRemap>>,
    ) -> LinderaResult<()> {
        UnknownDictionaryBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .context_id_remap(remap)
            .builder()
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?
            .build(input_dir, chardef, output_dir)
    }

    pub fn build_prefix_dictionary(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<Arc<ContextIdRemap>>,
    ) -> LinderaResult<()> {
        PrefixDictionaryBuilderOptions::default()
            .flexible_csv(self.metadata.flexible_csv)
            .encoding(self.metadata.encoding.clone())
            .skip_invalid_cost_or_id(self.metadata.skip_invalid_cost_or_id)
            .normalize_details(self.metadata.normalize_details)
            .schema(self.metadata.dictionary_schema.clone())
            .context_id_remap(remap)
            .builder()
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?
            .build(input_dir, output_dir)
    }

    pub fn build_connection_cost_matrix(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        remap: Option<Arc<ContextIdRemap>>,
    ) -> LinderaResult<()> {
        ConnectionCostMatrixBuilderOptions::default()
            .encoding(self.metadata.encoding.clone())
            .context_id_remap(remap)
            .builder()
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?
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
            .map_err(|err| LinderaErrorKind::Build.with_error(anyhow::anyhow!(err)))?
            .build(input_file)
    }
}

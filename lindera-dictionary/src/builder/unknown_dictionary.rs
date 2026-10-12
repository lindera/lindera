use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use log::debug;

use crate::LinderaResult;
use crate::builder::connection_cost_matrix::MatrixSize;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::unknown_dictionary::parse_unk_named;
use crate::error::LinderaErrorKind;
use crate::util::{read_file_with_encoding, write_data};

#[derive(Debug)]
pub struct UnknownDictionaryBuilder {
    encoding: Cow<'static, str>,
    /// Optional connection-cost context-ID remap, applied to each unknown-word
    /// entry's `left_id`/`right_id` so they match the remapped connection matrix.
    context_id_remap: Option<Arc<ContextIdMap>>,
    /// Axis sizes of the connection matrix. When present, every entry's context
    /// ids are checked against them before `unk.def` is parsed; see
    /// [`check_context_ids`]. `None` skips the check, which is what a stage run on
    /// its own gets: it does not read `matrix.def`.
    matrix_size: Option<MatrixSize>,
}

/// Options for [`UnknownDictionaryBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Debug, Default)]
pub struct UnknownDictionaryBuilderOptions {
    encoding: Option<Cow<'static, str>>,
    context_id_remap: Option<Arc<ContextIdMap>>,
    matrix_size: Option<MatrixSize>,
}

impl UnknownDictionaryBuilderOptions {
    pub fn encoding(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.encoding = Some(value.into());
        self
    }

    pub fn context_id_remap(&mut self, value: Option<Arc<ContextIdMap>>) -> &mut Self {
        self.context_id_remap = value;
        self
    }

    /// Sets the connection matrix size to check every entry's context ids
    /// against, as `mecab-dict-index` does.
    ///
    /// # Arguments
    ///
    /// * `value` - The axis sizes from the `matrix.def` header, or `None` (the
    ///   default) to skip the check.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    pub(crate) fn matrix_size(&mut self, value: Option<MatrixSize>) -> &mut Self {
        self.matrix_size = value;
        self
    }

    pub fn builder(&self) -> UnknownDictionaryBuilder {
        UnknownDictionaryBuilder {
            encoding: self.encoding.clone().unwrap_or_else(|| "UTF-8".into()),
            context_id_remap: self.context_id_remap.clone(),
            matrix_size: self.matrix_size,
        }
    }
}

impl UnknownDictionaryBuilder {
    pub fn build(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinition,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        let unk_data_path = input_dir.join("unk.def");
        debug!("reading {unk_data_path:?}");
        let unk_data = read_file_with_encoding(&unk_data_path, &self.encoding)?;
        if let Some(matrix_size) = self.matrix_size {
            check_context_ids(&unk_data, matrix_size, &unk_data_path)?;
        }
        let unknown_dictionary = parse_unk_named(
            &unk_data_path.display().to_string(),
            chardef.categories(),
            &unk_data,
            self.context_id_remap.as_deref(),
        )?;

        let mut unk_buffer = Vec::new();
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&unknown_dictionary).map_err(|err| {
            LinderaErrorKind::Serialize
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to serialize unknown dictionary data")
        })?;
        unk_buffer.write_all(&bytes).map_err(|err| {
            LinderaErrorKind::Io
                .with_error(anyhow::anyhow!(err))
                .add_context("Failed to write unknown dictionary data to buffer")
        })?;

        let wtr_unk_path = output_dir.join(Path::new("unk.bin"));
        let mut wtr_unk = io::BufWriter::new(
            File::create(wtr_unk_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        write_data(&unk_buffer, &mut wtr_unk)?;

        wtr_unk
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }
}

/// Checks the context ids of every `unk.def` entry against the connection
/// matrix size, as `mecab-dict-index` does, so that a dictionary whose ids
/// exceed its matrix fails to build instead of indexing past the matrix when
/// an unknown word is looked up.
///
/// The check runs on the raw lines, before [`parse_unk_named`] relabels the
/// ids through the remap and truncates them to `u16`: an id of 65537 is
/// rejected rather than read as 1. A line that the parser cannot read, with
/// fewer than four fields or an id that does not parse, is left to it to
/// report with the line.
///
/// # Arguments
///
/// * `unk_data` - The decoded `unk.def` content.
/// * `matrix_size` - The axis sizes from the `matrix.def` header.
/// * `path` - The `unk.def` path, for the message.
///
/// # Returns
///
/// `Ok(())`, or an error naming the file, the line, the offending id and the
/// matrix size.
fn check_context_ids(unk_data: &str, matrix_size: MatrixSize, path: &Path) -> LinderaResult<()> {
    for (index, line) in unk_data.lines().enumerate() {
        let line_number = index + 1;
        let fields: Vec<&str> = line.split(',').collect();
        let [_, left_id, right_id, _, ..] = fields[..] else {
            continue;
        };
        let (Ok(left_id), Ok(right_id)) = (u32::from_str(left_id), u32::from_str(right_id)) else {
            continue;
        };
        matrix_size
            .check_context_ids(left_id, right_id)
            .map_err(|message| {
                LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "{}: line {line_number}: {message}",
                    path.display()
                ))
            })?;
    }
    Ok(())
}

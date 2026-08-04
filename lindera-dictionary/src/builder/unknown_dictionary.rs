use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use log::debug;

use crate::LinderaResult;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::dictionary::unknown_dictionary::parse_unk;
use crate::error::LinderaErrorKind;
use crate::util::{read_file_with_encoding, write_data};

#[derive(Debug)]
pub struct UnknownDictionaryBuilder {
    encoding: Cow<'static, str>,
    /// Optional connection-cost context-ID remap, applied to each unknown-word
    /// entry's `left_id`/`right_id` so they match the remapped connection matrix.
    context_id_remap: Option<Arc<ContextIdMap>>,
}

/// Options for [`UnknownDictionaryBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Debug, Default)]
pub struct UnknownDictionaryBuilderOptions {
    encoding: Option<Cow<'static, str>>,
    context_id_remap: Option<Arc<ContextIdMap>>,
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

    pub fn builder(&self) -> UnknownDictionaryBuilder {
        UnknownDictionaryBuilder {
            encoding: self.encoding.clone().unwrap_or_else(|| "UTF-8".into()),
            context_id_remap: self.context_id_remap.clone(),
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
        let unknown_dictionary = parse_unk(
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

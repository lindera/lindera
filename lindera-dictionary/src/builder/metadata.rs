use std::path::Path;

use log::debug;

use crate::LinderaResult;
use crate::dictionary::metadata::Metadata;
use crate::error::LinderaErrorKind;

pub struct MetadataBuilder {}

impl MetadataBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(&self, metadata: &Metadata, output_dir: &Path) -> LinderaResult<()> {
        // Serialize to JSON with pretty formatting
        let json_data = serde_json::to_string_pretty(metadata)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;

        let metadata_path = output_dir.join("metadata.json");
        debug!("writing metadata to {metadata_path:?}");

        // Write JSON data to file
        std::fs::write(metadata_path, json_data)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

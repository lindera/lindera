use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use derive_builder::Builder;
use encoding_rs::Encoding;
use lindera_core::character_definition::{CharacterDefinitions, CharacterDefinitionsBuilder};
use lindera_core::error::LinderaErrorKind;
use lindera_core::file_util::read_file;
use lindera_core::LinderaResult;
use lindera_decompress::Algorithm;
use log::debug;

use crate::compress::compress_write;

#[derive(Builder, Debug)]
#[builder(name = "CharDefBuilderOptions")]
#[builder(build_fn(name = "builder"))]
pub struct CharDefBuilder {
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    #[builder(default = "Algorithm::Deflate")]
    compress_algorithm: Algorithm,
}

impl CharDefBuilder {
    pub fn build(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinitions> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {:?}", char_def_path);

        let encoding = Encoding::for_label_no_replacement(self.encoding.as_bytes());
        let encoding = encoding.ok_or_else(|| {
            LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", self.encoding))
        })?;

        let buffer = read_file(&char_def_path)?;
        let char_def = encoding.decode(&buffer).0;

        let mut char_definitions_builder = CharacterDefinitionsBuilder::default();
        char_definitions_builder.parse(&char_def)?;
        let char_definitions = char_definitions_builder.build();

        let mut chardef_buffer = Vec::new();
        bincode::serialize_into(&mut chardef_buffer, &char_definitions)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;

        let wtr_chardef_path = output_dir.join(Path::new("char_def.bin"));
        let mut wtr_chardef = io::BufWriter::new(
            File::create(wtr_chardef_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        compress_write(&chardef_buffer, self.compress_algorithm, &mut wtr_chardef)?;

        wtr_chardef
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(char_definitions)
    }
}

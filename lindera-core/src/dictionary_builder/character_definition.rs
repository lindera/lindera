use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use derive_builder::Builder;
use log::debug;

use crate::decompress::Algorithm;
use crate::dictionary::character_definition::{CharacterDefinition, CharacterDefinitionsBuilder};
use crate::dictionary_builder::utils::{compress_write, read_file_with_encoding};
use crate::error::LinderaErrorKind;
use crate::LinderaResult;

#[derive(Builder, Debug)]
#[builder(name = CharacterDefinitionBuilderOptions)]
#[builder(build_fn(name = "builder"))]
pub struct CharacterDefinitionBuilder {
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    #[builder(default = "Algorithm::Deflate")]
    compress_algorithm: Algorithm,
}

impl CharacterDefinitionBuilder {
    pub fn build(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<CharacterDefinition> {
        let char_def_path = input_dir.join("char.def");
        debug!("reading {:?}", char_def_path);
        let char_def = read_file_with_encoding(&char_def_path, &self.encoding)?;

        let mut char_definitions_builder = CharacterDefinitionsBuilder::default();
        char_definitions_builder.parse(&char_def)?;
        let char_definitions = char_definitions_builder.load();

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

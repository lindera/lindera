use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use derive_builder::Builder;
use log::debug;

use crate::decompress::Algorithm;
use crate::dictionary::character_definition::CharacterDefinition;
use crate::dictionary::unknown_dictionary::parse_unk;
use crate::error::LinderaErrorKind;
use crate::util::{compress_write, read_file_with_encoding};
use crate::LinderaResult;

#[derive(Builder, Debug)]
#[builder(name = UnknownDictionaryBuilderOptions)]
#[builder(build_fn(name = "builder"))]
pub struct UnknownDictionaryBuilder {
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    #[builder(default = "Algorithm::Deflate")]
    compress_algorithm: Algorithm,
    #[builder(default = "11")]
    unk_fields_num: usize,
}

impl UnknownDictionaryBuilder {
    pub fn build(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinition,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        let unk_data_path = input_dir.join("unk.def");
        debug!("reading {:?}", unk_data_path);
        let unk_data = read_file_with_encoding(&unk_data_path, &self.encoding)?;
        let unknown_dictionary = parse_unk(chardef.categories(), &unk_data, self.unk_fields_num)?;

        let mut unk_buffer = Vec::new();
        bincode::serialize_into(&mut unk_buffer, &unknown_dictionary)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;

        let wtr_unk_path = output_dir.join(Path::new("unk.bin"));
        let mut wtr_unk = io::BufWriter::new(
            File::create(wtr_unk_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        compress_write(&unk_buffer, self.compress_algorithm, &mut wtr_unk)?;

        wtr_unk
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }
}

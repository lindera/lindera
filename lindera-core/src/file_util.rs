use std::fs::File;
use std::io::Read;
use std::path::Path;

use encoding::{DecoderTrap, Encoding};

use crate::error::LinderaErrorKind;
use crate::LinderaResult;

pub fn read_file(filename: &Path) -> LinderaResult<Vec<u8>> {
    let mut input_read = File::open(filename)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let mut buffer = Vec::new();
    input_read
        .read_to_end(&mut buffer)
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    Ok(buffer)
}

pub fn read_euc_file(filename: &Path) -> LinderaResult<String> {
    let buffer = read_file(filename)?;
    encoding::all::EUC_JP
        .decode(&buffer, DecoderTrap::Strict)
        .map_err(|err| LinderaErrorKind::Decode.with_error(anyhow::anyhow!(err)))
}

pub fn read_utf8_file(filename: &Path) -> LinderaResult<String> {
    let buffer = read_file(filename)?;
    encoding::all::UTF_8
        .decode(&buffer, DecoderTrap::Strict)
        .map_err(|err| LinderaErrorKind::Decode.with_error(anyhow::anyhow!(err)))
}

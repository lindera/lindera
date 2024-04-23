use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use byteorder::{LittleEndian, WriteBytesExt};
use derive_builder::Builder;
use encoding_rs::Encoding;
use lindera_core::error::LinderaErrorKind;
use lindera_core::file_util::read_file;
use lindera_core::LinderaResult;
use lindera_decompress::Algorithm;
use log::debug;

use crate::compress::compress_write;

#[derive(Builder, Debug)]
#[builder(name = "CostMatrixBuilderOptions")]
#[builder(build_fn(name = "builder"))]
pub struct CostMatrixBuilder {
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    #[builder(default = "Algorithm::Deflate")]
    compress_algorithm: Algorithm,
}

impl CostMatrixBuilder {
    pub fn build(&self, matrix_data_path: &Path, output_dir: &Path) -> LinderaResult<()> {
        debug!("reading {:?}", matrix_data_path);

        let encoding = Encoding::for_label_no_replacement(self.encoding.as_bytes());
        let encoding = encoding.ok_or_else(|| {
            LinderaErrorKind::Decode.with_error(anyhow!("Invalid encoding: {}", self.encoding))
        })?;

        let buffer = read_file(matrix_data_path)?;
        let matrix_data = encoding.decode(&buffer).0;

        let mut lines = Vec::new();
        for line in matrix_data.lines() {
            let fields: Vec<i32> = line
                .split_whitespace()
                .map(i32::from_str)
                .collect::<Result<_, _>>()
                .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
            lines.push(fields);
        }
        let mut lines_it = lines.into_iter();
        let header = lines_it.next().ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
        })?;
        let forward_size = header[0] as u32;
        let backward_size = header[1] as u32;
        let len = 2 + (forward_size * backward_size) as usize;
        let mut costs = vec![i16::MAX; len];
        costs[0] = forward_size as i16;
        costs[1] = backward_size as i16;
        for fields in lines_it {
            let forward_id = fields[0] as u32;
            let backward_id = fields[1] as u32;
            let cost = fields[2] as u16;
            costs[2 + (backward_id + forward_id * backward_size) as usize] = cost as i16;
        }

        let wtr_matrix_mtx_path = output_dir.join(Path::new("matrix.mtx"));
        let mut wtr_matrix_mtx = io::BufWriter::new(
            File::create(wtr_matrix_mtx_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        let mut matrix_mtx_buffer = Vec::new();
        for cost in costs {
            matrix_mtx_buffer
                .write_i16::<LittleEndian>(cost)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        }

        compress_write(
            &matrix_mtx_buffer,
            self.compress_algorithm,
            &mut wtr_matrix_mtx,
        )?;

        wtr_matrix_mtx
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }
}

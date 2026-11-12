use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
#[cfg(not(target_family = "wasm"))]
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::debug;
use memchr::{memchr, memrchr};

use crate::LinderaResult;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::error::{LinderaError, LinderaErrorKind};
use crate::util::write_data;

/// UTF-8 byte order mark. `encoding_rs::Encoding::decode` strips a leading
/// UTF-8 BOM, so the raw-byte fast path must strip it too to stay
/// byte-identical with the previous decode-based implementation.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Minimum `matrix.def` data size (in bytes, excluding the header line) at
/// which the parser switches to the parallel path. Small connection matrices
/// (e.g. CC-CEDICT's 1x1, Jieba's 56x56) are parsed sequentially to avoid
/// thread-pool overhead.
#[cfg(not(target_family = "wasm"))]
const PARALLEL_THRESHOLD: usize = 1 << 20; // 1 MiB

/// Size of the blocks `matrix.def` is read and parsed in (#1366).
///
/// The builder holds one block of the text at a time, besides the matrix,
/// rather than the whole file: UniDic CWJ's `matrix.def` is 7.70 GiB of text
/// for a 982 MiB matrix. A block is far above the parallel threshold, so a
/// large matrix is still parsed in parallel, one block after another.
///
/// A block is no larger than the file, so a smaller `matrix.def` (IPADIC's
/// is 22 MiB) is read into a buffer of its own size, not of 64 MiB.
const BLOCK_SIZE: usize = 64 << 20; // 64 MiB

/// The smallest block `matrix.def` is read in, whatever size the file
/// reports: a file whose size is not known (a pipe reports 0 bytes) is still
/// read in blocks of a useful size rather than byte by byte.
const MIN_BLOCK_SIZE: usize = 64 << 10; // 64 KiB

/// Number of matrix values converted to bytes at a time when `matrix.mtx` is
/// written, so that the file is written without a byte copy of the matrix.
const WRITE_CHUNK_VALUES: usize = 1 << 16;

/// The cost of a cell that `matrix.def` does not list.
///
/// A `matrix.def` need not list every `(forward_id, backward_id)` pair. A
/// pair it leaves out costs 0, as in MeCab's `mecab-dict-index`, whose
/// `Connector::compile` fills the matrix with 0 before it reads the file.
/// Up to v6.2.0 such a cell cost `i16::MAX` (#1211).
const MISSING_CELL_COST: i16 = 0;

/// The axis sizes declared by the `matrix.def` header, `forward_size
/// backward_size`.
///
/// The runtime looks a connection up as
/// `row(backward = right word's left_id)[forward = left word's right_id]`, so
/// every right context id must be below `forward_size` and every left context
/// id below `backward_size`. MeCab's `mecab-dict-index` rejects a dictionary
/// entry that breaks either bound (`Connector::is_valid`); without the check,
/// such an entry builds and then indexes past the matrix when it is looked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatrixSize {
    /// Number of forward ids, the first header value: the right context ids
    /// of the left word.
    pub(crate) forward_size: u32,
    /// Number of backward ids, the second header value: the left context ids
    /// of the right word.
    pub(crate) backward_size: u32,
}

impl MatrixSize {
    /// Takes the axis sizes of a built connection matrix, such as the one of
    /// the system dictionary a user dictionary is checked against.
    ///
    /// # Arguments
    ///
    /// * `matrix` - The connection matrix.
    ///
    /// # Returns
    ///
    /// The matrix's forward and backward sizes.
    pub(crate) fn from_matrix(matrix: &ConnectionCostMatrix) -> Self {
        Self {
            forward_size: matrix.forward_size,
            backward_size: matrix.backward_size,
        }
    }

    /// Checks that a dictionary entry's context ids address a cell of the
    /// connection matrix.
    ///
    /// The ids are those written in the source files, before any context-id
    /// remap, and are taken as `u32` so that an id too large for the `u16`
    /// the dictionary stores is not truncated into range first.
    ///
    /// # Arguments
    ///
    /// * `left_id` - The entry's left context id.
    /// * `right_id` - The entry's right context id.
    ///
    /// # Returns
    ///
    /// `Ok(())` when both ids are in range, or `Err` with a message naming
    /// the first offending id and the matrix size. The message carries no
    /// location, which the caller adds.
    pub(crate) fn check_context_ids(&self, left_id: u32, right_id: u32) -> Result<(), String> {
        if left_id >= self.backward_size {
            return Err(format!(
                "left context id {left_id} is out of range for the connection matrix \
                 (matrix.def size {} {}; left context ids must be below {})",
                self.forward_size, self.backward_size, self.backward_size
            ));
        }
        if right_id >= self.forward_size {
            return Err(format!(
                "right context id {right_id} is out of range for the connection matrix \
                 (matrix.def size {} {}; right context ids must be below {})",
                self.forward_size, self.backward_size, self.forward_size
            ));
        }
        Ok(())
    }
}

/// Builder for the connection cost matrix (`matrix.mtx`).
#[derive(Debug)]
pub struct ConnectionCostMatrixBuilder {
    /// Character encoding of the source `matrix.def` file.
    ///
    /// If set to UTF-8, files with a UTF-16 BOM are still decoded correctly.
    encoding: Cow<'static, str>,
    /// Optional connection-cost context-ID remapping. When present, `forward_id`
    /// (right-context id) is mapped through `remap.right` and `backward_id`
    /// (left-context id) through `remap.left` before the cost is scattered, so
    /// frequently-used cells cluster near the front of each row. `None` keeps the
    /// output byte-identical to the un-remapped build.
    context_id_remap: Option<Arc<ContextIdMap>>,
}

/// Options for [`ConnectionCostMatrixBuilder`]. Every field has a default, so
/// [`Self::builder`] is infallible.
#[derive(Debug, Default)]
pub struct ConnectionCostMatrixBuilderOptions {
    encoding: Option<Cow<'static, str>>,
    context_id_remap: Option<Arc<ContextIdMap>>,
}

impl ConnectionCostMatrixBuilderOptions {
    pub fn encoding(&mut self, value: impl Into<Cow<'static, str>>) -> &mut Self {
        self.encoding = Some(value.into());
        self
    }

    pub fn context_id_remap(&mut self, value: Option<Arc<ContextIdMap>>) -> &mut Self {
        self.context_id_remap = value;
        self
    }

    pub fn builder(&self) -> ConnectionCostMatrixBuilder {
        ConnectionCostMatrixBuilder {
            encoding: self.encoding.clone().unwrap_or_else(|| "UTF-8".into()),
            context_id_remap: self.context_id_remap.clone(),
        }
    }
}

impl ConnectionCostMatrixBuilder {
    /// Build `matrix.mtx` from the `matrix.def` file in `input_dir`.
    ///
    /// A cell that `matrix.def` does not list costs 0, as in MeCab's
    /// `mecab-dict-index`.
    ///
    /// The file is read in blocks of 64 MiB, or of the file's size if that is
    /// smaller, so the build holds the matrix and one block of the text rather
    /// than the whole text (#1366). The parser reads the raw bytes and, for
    /// ASCII-compatible encodings, parses the space-separated integers
    /// directly. A file whose encoding is UTF-16 (by configured label or by
    /// BOM) is decoded to UTF-8 as it is read. On non-wasm targets each block
    /// of a large matrix is parsed in parallel with rayon.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the source `matrix.def`.
    /// * `output_dir` - Directory to write `matrix.mtx` into.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`LinderaResult`] error if the file cannot be
    /// read, the header is missing, or a data line is malformed. Nothing is
    /// written then.
    pub fn build(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        let matrix_data_path = input_dir.join("matrix.def");
        debug!("reading {matrix_data_path:?}");
        let file = File::open(&matrix_data_path).map_err(|err| {
            LinderaErrorKind::Io.with_error(err).add_context(format!(
                "Failed to open file: {}",
                matrix_data_path.display()
            ))
        })?;
        // A block need not be larger than the file: the text after the header
        // is shorter, and decoding UTF-16 halves it.
        let file_len = file.metadata().map_err(read_error)?.len();
        let block_size = file_len.clamp(MIN_BLOCK_SIZE as u64, BLOCK_SIZE as u64) as usize;
        let mut reader = self.open_matrix_reader(file)?;

        // Parse the header line ("<forward_size> <backward_size>").
        let mut header = Vec::new();
        reader.read_until(b'\n', &mut header).map_err(read_error)?;
        let header = header.strip_suffix(b"\n").unwrap_or(&header);
        let matrix_size = parse_header(strip_utf8_bom(header))?;
        let MatrixSize {
            forward_size,
            backward_size,
        } = matrix_size;

        // Guard against a remap whose axis sizes diverge from the matrix header:
        // a shifted index would scatter costs to the wrong cells and silently
        // corrupt every connection cost.
        if let Some(remap) = self.context_id_remap.as_deref()
            && (remap.right.len() != forward_size as usize
                || remap.left.len() != backward_size as usize)
        {
            return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "context-id remap size mismatch: remap.right={} vs forward_size={}, remap.left={} vs backward_size={}",
                remap.right.len(),
                forward_size,
                remap.left.len(),
                backward_size
            )));
        }

        let mut costs = initial_costs(matrix_size);

        // Parse the data region (everything after the header line) and scatter
        // the costs into `costs`. Applied in file order so a duplicated
        // (forward_id, backward_id) pair keeps last-occurrence-wins semantics.
        self.fill_costs(&mut reader, block_size, matrix_size, &mut costs)?;

        write_matrix_mtx(&costs, &output_dir.join(Path::new("matrix.mtx")))
    }

    /// Opens `matrix.def` for parsing as ASCII-compatible bytes.
    ///
    /// `matrix.def` holds only ASCII, so a file in an ASCII-compatible
    /// encoding is read as it is, and its UTF-8 BOM, if any, is left for the
    /// header to drop. A file whose configured encoding is UTF-16, or that
    /// starts with a UTF-16 BOM, is decoded to UTF-8 as it is read; its BOM
    /// takes precedence over the label and is dropped, as `Encoding::decode`
    /// does.
    ///
    /// # Arguments
    ///
    /// * `file` - The opened `matrix.def`.
    ///
    /// # Returns
    ///
    /// A reader of the file's bytes, or an error if the encoding label is
    /// invalid or the start of the file cannot be read.
    fn open_matrix_reader(&self, file: File) -> LinderaResult<Box<dyn BufRead>> {
        let encoding =
            Encoding::for_label_no_replacement(self.encoding.as_bytes()).ok_or_else(|| {
                LinderaErrorKind::Decode
                    .with_error(anyhow::anyhow!("Invalid encoding: {}", self.encoding))
            })?;

        let mut reader = BufReader::new(file);
        let head = reader.fill_buf().map_err(read_error)?;
        let is_utf16 = encoding == UTF_16LE || encoding == UTF_16BE || has_utf16_bom(head);
        if is_utf16 {
            let decoder = DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding))
                .bom_override(true)
                .strip_bom(true)
                .build(reader);
            Ok(Box::new(BufReader::new(decoder)))
        } else {
            Ok(Box::new(reader))
        }
    }

    /// Parse the data region and scatter costs into `costs`.
    ///
    /// On non-wasm targets a block large enough is parsed in parallel.
    ///
    /// # Arguments
    ///
    /// * `reader` - The file, positioned after the header line.
    /// * `block_size` - The size of the blocks to read: [`BLOCK_SIZE`], or the
    ///   file's size if that is smaller, but any size in tests.
    /// * `matrix_size` - The axis sizes from the header; every entry is
    ///   checked against them.
    /// * `costs` - Destination cost array to scatter into.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error naming the line of the first malformed or
    /// out-of-range entry found.
    #[cfg(not(target_family = "wasm"))]
    fn fill_costs(
        &self,
        reader: &mut dyn BufRead,
        block_size: usize,
        matrix_size: MatrixSize,
        costs: &mut [i16],
    ) -> LinderaResult<()> {
        let remap = self.context_id_remap.as_deref();
        for_each_block(reader, block_size, |block, first_line| {
            if block.len() >= PARALLEL_THRESHOLD {
                fill_costs_parallel(block, first_line, matrix_size, costs, remap)
            } else {
                fill_costs_sequential(block, first_line, matrix_size, costs, remap)
            }
        })
    }

    /// Parse the data region and scatter costs into `costs` (wasm: always
    /// sequential, since rayon is unavailable on `wasm32-unknown-unknown`).
    ///
    /// # Arguments
    ///
    /// * `reader` - The file, positioned after the header line.
    /// * `block_size` - The size of the blocks to read: [`BLOCK_SIZE`], or the
    ///   file's size if that is smaller, but any size in tests.
    /// * `matrix_size` - The axis sizes from the header; every entry is
    ///   checked against them.
    /// * `costs` - Destination cost array to scatter into.
    ///
    /// # Returns
    ///
    /// `Ok(())`, or an error naming the line of the first malformed or
    /// out-of-range entry.
    #[cfg(target_family = "wasm")]
    fn fill_costs(
        &self,
        reader: &mut dyn BufRead,
        block_size: usize,
        matrix_size: MatrixSize,
        costs: &mut [i16],
    ) -> LinderaResult<()> {
        let remap = self.context_id_remap.as_deref();
        for_each_block(reader, block_size, |block, first_line| {
            fill_costs_sequential(block, first_line, matrix_size, costs, remap)
        })
    }
}

/// Reads the data region of `matrix.def` in blocks of at most `block_size`
/// bytes that end at a line end, and hands each block to `parse` with the
/// number of its first line in the file.
///
/// The part of a line that a block cuts off is carried over to the start of
/// the next block, so `parse` never sees part of a line. A line longer than a
/// block makes the block grow until the line ends. The last block ends at the
/// end of the file, with or without a line end.
///
/// # Arguments
///
/// * `reader` - The file, positioned after the header line.
/// * `block_size` - The most bytes a block holds, unless one line is longer.
/// * `parse` - Parses one block; returns the number of lines it went through,
///   which numbers the lines of the next block.
///
/// # Returns
///
/// `Ok(())`, or the first error of `parse` or of the read.
fn for_each_block(
    reader: &mut dyn BufRead,
    block_size: usize,
    mut parse: impl FnMut(&[u8], usize) -> LinderaResult<usize>,
) -> LinderaResult<()> {
    // A block of 0 bytes would read nothing and never end.
    let block_size = block_size.max(1);
    let mut block = Vec::with_capacity(block_size);
    // The header is line 1, so the data region starts on line 2.
    let mut first_line = 2;
    loop {
        // Fill the block up to its size after the carried-over line, or by
        // another block's size while one line is longer than a block.
        let want = match block_size.checked_sub(block.len()) {
            Some(room) if room > 0 => room,
            _ => block_size,
        };
        let read = Read::take(&mut *reader, want as u64)
            .read_to_end(&mut block)
            .map_err(read_error)?;
        // `Take` stops early only at the end of the file.
        let at_end = read < want;
        let end = if at_end {
            block.len()
        } else {
            match memrchr(b'\n', &block) {
                Some(last_newline) => last_newline + 1,
                // No line end yet: read on until the line ends.
                None => continue,
            }
        };
        if end > 0 {
            first_line += parse(&block[..end], first_line)?;
        }
        if at_end {
            return Ok(());
        }
        block.drain(..end);
    }
}

/// Maps a failed read of `matrix.def` to a Lindera error.
///
/// # Arguments
///
/// * `err` - The I/O error of the read.
///
/// # Returns
///
/// An `Io` error saying that `matrix.def` could not be read.
fn read_error(err: io::Error) -> LinderaError {
    LinderaErrorKind::Io
        .with_error(err)
        .add_context("Failed to read matrix.def")
}

/// Writes the matrix values to `path` as little-endian `i16`s.
///
/// The values are converted [`WRITE_CHUNK_VALUES`] at a time, so the file is
/// written without a byte copy of the whole matrix (#1366).
///
/// # Arguments
///
/// * `costs` - The `matrix.mtx` values: the header and the cells.
/// * `path` - Where to write `matrix.mtx`.
///
/// # Returns
///
/// `Ok(())`, or an error if the file cannot be created or written.
fn write_matrix_mtx(costs: &[i16], path: &Path) -> LinderaResult<()> {
    let file =
        File::create(path).map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
    let mut writer = BufWriter::new(file);
    let mut bytes = Vec::with_capacity(WRITE_CHUNK_VALUES * 2);
    for chunk in costs.chunks(WRITE_CHUNK_VALUES) {
        bytes.clear();
        for cost in chunk {
            bytes.extend_from_slice(&cost.to_le_bytes());
        }
        write_data(&bytes, &mut writer)?;
    }
    writer
        .flush()
        .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
}

/// Creates the `matrix.mtx` values before `matrix.def` is read: the
/// transposed-layout header followed by every cell at [`MISSING_CELL_COST`].
///
/// The header is the version flag `-1` and the two axis sizes. The cells
/// that `matrix.def` lists then overwrite their initial cost.
///
/// # Arguments
///
/// * `matrix_size` - The axis sizes from the `matrix.def` header.
///
/// # Returns
///
/// `3 + forward_size * backward_size` values.
fn initial_costs(matrix_size: MatrixSize) -> Vec<i16> {
    let MatrixSize {
        forward_size,
        backward_size,
    } = matrix_size;
    let len = 3 + (forward_size as usize) * (backward_size as usize);
    let mut costs = vec![MISSING_CELL_COST; len];
    costs[0] = -1; // Version flag for transposed layout
    costs[1] = forward_size as i16;
    costs[2] = backward_size as i16;
    costs
}

/// Return the buffer with a leading UTF-8 BOM removed, if present.
///
/// # Arguments
///
/// * `buffer` - The raw bytes read from `matrix.def`.
///
/// # Returns
///
/// The buffer without a leading UTF-8 BOM.
fn strip_utf8_bom(buffer: &[u8]) -> &[u8] {
    buffer.strip_prefix(UTF8_BOM).unwrap_or(buffer)
}

/// Return `true` if the buffer starts with a UTF-16 (LE or BE) byte order
/// mark. A UTF-32 LE BOM (`FF FE 00 00`) also starts with the UTF-16 LE BOM
/// and is likewise routed to the decode path.
///
/// # Arguments
///
/// * `buffer` - The raw bytes read from `matrix.def`.
///
/// # Returns
///
/// `true` when a UTF-16 BOM is present.
fn has_utf16_bom(buffer: &[u8]) -> bool {
    buffer.starts_with(&[0xFF, 0xFE]) || buffer.starts_with(&[0xFE, 0xFF])
}

/// Read only the `<forward_size> <backward_size>` header from `matrix.def` without
/// loading the whole (potentially huge) matrix body.
///
/// The file is decoded as [`ConnectionCostMatrixBuilder::build`] decodes it: a BOM
/// takes precedence over `encoding`, so a UTF-16 `matrix.def` is read whether it is
/// labeled UTF-16 or carries a UTF-16 BOM. Only the first line is decoded.
///
/// This is the single source of truth for the connection-matrix axis sizes used by
/// [`super::context_id_remap::compute_context_id_remap`] and by the context-id checks
/// of [`super::DictionaryBuilder::build_dictionary`], so both work on the same axes
/// the matrix build scatters into.
///
/// # Arguments
///
/// * `input_dir` - Directory containing `matrix.def`.
/// * `encoding` - Source encoding label of the dictionary.
///
/// # Returns
///
/// The axis sizes, or an error if the encoding label is invalid, the file is missing,
/// or the header is malformed.
pub(crate) fn read_matrix_header(input_dir: &Path, encoding: &str) -> LinderaResult<MatrixSize> {
    let encoding = Encoding::for_label_no_replacement(encoding.as_bytes()).ok_or_else(|| {
        LinderaErrorKind::Decode.with_error(anyhow::anyhow!("Invalid encoding: {encoding}"))
    })?;
    let path = input_dir.join("matrix.def");
    let file = File::open(&path).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to open matrix.def: {path:?}"))
    })?;
    // `bom_override` makes a BOM win over the label, as `Encoding::decode` does in
    // the matrix stage.
    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .bom_override(true)
        .build(file);
    let mut reader = io::BufReader::new(decoder);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context("Failed to read matrix.def header line")
    })?;
    parse_header(strip_utf8_bom(&line))
}

/// Parse the `matrix.def` header line, `<forward_size> <backward_size>`.
///
/// # Arguments
///
/// * `header` - The header line bytes, without a BOM.
///
/// # Returns
///
/// The axis sizes, or an error if either size is missing.
fn parse_header(header: &[u8]) -> LinderaResult<MatrixSize> {
    let mut pos = 0;
    let forward_size = next_int(header, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content
            .with_error(anyhow::anyhow!("matrix.def is missing the size header"))
    })? as u32;
    let backward_size = next_int(header, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "matrix.def header is missing backward size"
        ))
    })? as u32;
    Ok(MatrixSize {
        forward_size,
        backward_size,
    })
}

/// Parse the next whitespace-delimited signed integer from `bytes`, advancing
/// `pos` past it. Leading spaces, tabs, and carriage returns are skipped.
///
/// # Arguments
///
/// * `bytes` - The line (or header) bytes to parse.
/// * `pos` - Cursor into `bytes`, advanced past the parsed integer.
///
/// # Returns
///
/// `Some(value)` if an integer was parsed, or `None` if no digits remain
/// (e.g. an empty or whitespace-only line).
fn next_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    while *pos < bytes.len() && matches!(bytes[*pos], b' ' | b'\t' | b'\r') {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let negative = bytes[*pos] == b'-';
    if negative {
        *pos += 1;
    }
    let start = *pos;
    let mut value: i32 = 0;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        // matrix.def integers are small (context IDs < ~6000, costs within
        // i16 range), so wrapping arithmetic never triggers in practice; it
        // only avoids a debug-mode panic on pathological input.
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[*pos] - b'0') as i32);
        *pos += 1;
    }
    if *pos == start {
        // A lone '-' with no digits: not a valid integer.
        return None;
    }
    Some(if negative { -value } else { value })
}

/// Parse a single `matrix.def` data line into a `(index, cost)` pair.
///
/// Casts match the previous implementation exactly (`cost` via
/// `i32 as u16 as i16`) so the resulting bytes are identical.
///
/// Each id is checked against its own axis, as MeCab's `mecab-dict-index`
/// does (`CHECK_DIE(l < lsize && r < rsize)` in `connector.cpp`). Checking only
/// the flat index let an entry whose forward id is past its axis land in a
/// cell of the next row.
///
/// # Arguments
///
/// * `line` - The line bytes (without the trailing newline).
/// * `matrix_size` - The axis sizes from the header.
/// * `costs_len` - Length of the destination cost array, for bounds checking.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// `Ok(Some((index, cost)))` for a data line, `Ok(None)` for an empty or
/// whitespace-only line, or `Err` with a message (without the line number,
/// which the caller adds) for a malformed or out-of-range line.
fn parse_data_line(
    line: &[u8],
    matrix_size: MatrixSize,
    costs_len: usize,
    remap: Option<&ContextIdMap>,
) -> Result<Option<(usize, i16)>, String> {
    let MatrixSize {
        forward_size,
        backward_size,
    } = matrix_size;
    let mut pos = 0;
    let Some(forward_id) = next_int(line, &mut pos) else {
        // Empty or whitespace-only line: skip it.
        return Ok(None);
    };
    let backward_id =
        next_int(line, &mut pos).ok_or_else(|| "the line is missing backward id".to_string())?;
    let cost = next_int(line, &mut pos).ok_or_else(|| "the line is missing cost".to_string())?;

    let in_range = |id: i32, size: u32| u32::try_from(id).is_ok_and(|id| id < size);
    if !in_range(forward_id, forward_size) || !in_range(backward_id, backward_size) {
        return Err(format!(
            "entry ({forward_id}, {backward_id}) is out of range for the matrix size \
             {forward_size} {backward_size} (forward ids must be below {forward_size}, \
             backward ids below {backward_size})"
        ));
    }
    let forward_id = forward_id as usize;
    let backward_id = backward_id as usize;
    // Apply the frequency remap (right-context id via P_right, left-context id via
    // P_left). Sizes are guaranteed equal to the axes by the guard in `build`, so
    // after the axis check above this and the index check below cannot fail; they
    // stay as guards against an inconsistent caller.
    let (fwd, bwd) = match remap {
        Some(m) => {
            if forward_id >= m.right.len() || backward_id >= m.left.len() {
                return Err(format!(
                    "entry ({forward_id}, {backward_id}) is out of range of the context-id remap"
                ));
            }
            (m.right[forward_id] as usize, m.left[backward_id] as usize)
        }
        None => (forward_id, backward_id),
    };
    let index = 3 + fwd + bwd * forward_size as usize;
    if index >= costs_len {
        return Err(format!(
            "entry ({forward_id}, {backward_id}) is out of range"
        ));
    }
    let cost = (cost as u16) as i16;
    Ok(Some((index, cost)))
}

/// Builds the error for a malformed `matrix.def` data line, naming its line
/// number.
///
/// The number is counted only on this error path, so the parsers do not keep
/// track of it for every line.
///
/// # Arguments
///
/// * `data` - The block of the data region the line is in.
/// * `first_line` - The line number of the block's first line in the file.
/// * `line_start` - Offset of the line's first byte in `data`.
/// * `message` - What is wrong with the line.
///
/// # Returns
///
/// A `Content` error reading `matrix.def line <n>: <message>`.
fn data_line_error(
    data: &[u8],
    first_line: usize,
    line_start: usize,
    message: String,
) -> LinderaError {
    let line = first_line + memchr::memchr_iter(b'\n', &data[..line_start]).count();
    LinderaErrorKind::Content.with_error(anyhow::anyhow!("matrix.def line {line}: {message}"))
}

/// Parse a block of the data region sequentially, scattering costs into
/// `costs`.
///
/// # Arguments
///
/// * `data` - A block of the data region, ending at a line end or at the end
///   of the file.
/// * `first_line` - The line number of the block's first line in the file.
/// * `matrix_size` - The axis sizes from the header.
/// * `costs` - Destination cost array to scatter into.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// The number of lines in the block, or an error naming the line of the
/// first malformed or out-of-range entry.
fn fill_costs_sequential(
    data: &[u8],
    first_line: usize,
    matrix_size: MatrixSize,
    costs: &mut [i16],
    remap: Option<&ContextIdMap>,
) -> LinderaResult<usize> {
    let costs_len = costs.len();
    let mut lines = 0;
    let mut pos = 0;
    while pos < data.len() {
        let line_end = memchr(b'\n', &data[pos..])
            .map(|offset| pos + offset)
            .unwrap_or(data.len());
        match parse_data_line(&data[pos..line_end], matrix_size, costs_len, remap) {
            Ok(Some((index, cost))) => costs[index] = cost,
            Ok(None) => {}
            Err(message) => return Err(data_line_error(data, first_line, pos, message)),
        }
        lines += 1;
        pos = line_end + 1;
    }
    Ok(lines)
}

/// Parse a block of the data region in parallel, then scatter costs into
/// `costs` in file order (preserving last-occurrence-wins for duplicate
/// entries).
///
/// The block is split into newline-aligned chunks, each parsed on a rayon
/// worker into a `(index, cost)` list; the lists are then applied in order.
///
/// # Arguments
///
/// * `data` - A block of the data region, ending at a line end or at the end
///   of the file.
/// * `first_line` - The line number of the block's first line in the file.
/// * `matrix_size` - The axis sizes from the header.
/// * `costs` - Destination cost array to scatter into.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// The number of lines in the block, or an error naming the line of a
/// malformed or out-of-range entry.
#[cfg(not(target_family = "wasm"))]
fn fill_costs_parallel(
    data: &[u8],
    first_line: usize,
    matrix_size: MatrixSize,
    costs: &mut [i16],
    remap: Option<&ContextIdMap>,
) -> LinderaResult<usize> {
    use rayon::prelude::*;

    let costs_len = costs.len();
    let n_chunks = (rayon::current_num_threads() * 4).max(1);

    // Compute chunk boundaries snapped to line starts so no line is split.
    let mut bounds = Vec::with_capacity(n_chunks + 1);
    bounds.push(0usize);
    for i in 1..n_chunks {
        let target = data.len() * i / n_chunks;
        let last = *bounds.last().unwrap_or(&0);
        if target <= last {
            continue;
        }
        if let Some(offset) = memchr(b'\n', &data[target..]) {
            let boundary = target + offset + 1;
            if boundary > last && boundary < data.len() {
                bounds.push(boundary);
            }
        }
    }
    bounds.push(data.len());

    let chunks: Vec<Range<usize>> = bounds.windows(2).map(|w| w[0]..w[1]).collect();
    let partials: Vec<(Vec<(usize, i16)>, usize)> = chunks
        .par_iter()
        .map(|chunk| {
            parse_chunk(
                data,
                first_line,
                chunk.clone(),
                matrix_size,
                costs_len,
                remap,
            )
        })
        .collect::<LinderaResult<Vec<_>>>()?;

    let mut lines = 0;
    for (partial, chunk_lines) in &partials {
        for &(index, cost) in partial {
            costs[index] = cost;
        }
        lines += chunk_lines;
    }
    Ok(lines)
}

/// Parse all data lines in a single chunk into a `(index, cost)` list.
///
/// # Arguments
///
/// * `data` - The block of the data region, which the line numbers in errors
///   are counted over.
/// * `first_line` - The line number of the block's first line in the file.
/// * `chunk` - The newline-aligned range of `data` to parse.
/// * `matrix_size` - The axis sizes from the header.
/// * `costs_len` - Length of the destination cost array, for bounds checking.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// The parsed `(index, cost)` pairs in chunk order and the number of lines
/// in the chunk, or an error naming the line of the first malformed or
/// out-of-range entry in the chunk.
#[cfg(not(target_family = "wasm"))]
fn parse_chunk(
    data: &[u8],
    first_line: usize,
    chunk: Range<usize>,
    matrix_size: MatrixSize,
    costs_len: usize,
    remap: Option<&ContextIdMap>,
) -> LinderaResult<(Vec<(usize, i16)>, usize)> {
    let mut out = Vec::with_capacity(chunk.len() / 8);
    let mut lines = 0;
    let mut pos = chunk.start;
    while pos < chunk.end {
        let line_end = memchr(b'\n', &data[pos..chunk.end])
            .map(|offset| pos + offset)
            .unwrap_or(chunk.end);
        match parse_data_line(&data[pos..line_end], matrix_size, costs_len, remap) {
            Ok(Some(entry)) => out.push(entry),
            Ok(None) => {}
            Err(message) => return Err(data_line_error(data, first_line, pos, message)),
        }
        lines += 1;
        pos = line_end + 1;
    }
    Ok((out, lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference parser replicating the previous split_whitespace + from_str
    /// implementation, used to assert byte-identical output. A cell that the
    /// matrix does not list is 0, as in `mecab-dict-index`, spelled here
    /// independently of [`MISSING_CELL_COST`].
    fn reference_costs(matrix: &str) -> Vec<i16> {
        let mut lines = Vec::new();
        for line in matrix.lines() {
            let fields: Vec<i32> = line
                .split_whitespace()
                .map(|f| f.parse::<i32>().unwrap())
                .collect();
            lines.push(fields);
        }
        let mut lines_it = lines.into_iter();
        let header = lines_it.next().unwrap();
        let forward_size = header[0] as u32;
        let backward_size = header[1] as u32;
        let len = 3 + (forward_size * backward_size) as usize;
        let mut costs = vec![0; len];
        costs[0] = -1;
        costs[1] = forward_size as i16;
        costs[2] = backward_size as i16;
        for fields in lines_it {
            if fields.is_empty() {
                continue;
            }
            let forward_id = fields[0] as u32;
            let backward_id = fields[1] as u32;
            let cost = fields[2] as u16;
            costs[3 + (forward_id + backward_id * forward_size) as usize] = cost as i16;
        }
        costs
    }

    /// Parse a matrix string through the new code path under test.
    fn new_costs(matrix: &str) -> Vec<i16> {
        let bytes = matrix.as_bytes();
        let header_end = memchr(b'\n', bytes).unwrap_or(bytes.len());
        let mut header_pos = 0;
        let forward_size = next_int(&bytes[..header_end], &mut header_pos).unwrap() as u32;
        let backward_size = next_int(&bytes[..header_end], &mut header_pos).unwrap() as u32;
        let matrix_size = MatrixSize {
            forward_size,
            backward_size,
        };
        let mut costs = initial_costs(matrix_size);
        let data = if header_end < bytes.len() {
            &bytes[header_end + 1..]
        } else {
            &[]
        };
        fill_costs_sequential(data, 2, matrix_size, &mut costs, None).unwrap();
        costs
    }

    #[test]
    fn test_matches_reference_simple() {
        // 2x2 matrix, all cells present, extra whitespace, trailing newline.
        let matrix = "2 2\n0 0 10\n0 1 20\n1 0 30\n1 1 40\n";
        assert_eq!(new_costs(matrix), reference_costs(matrix));
    }

    #[test]
    fn test_matches_reference_sparse_and_negative() {
        // Missing cells cost 0; negative cost round-trips via the u16 cast;
        // multiple spaces between fields.
        let matrix = "3 2\n0  0  -1\n2 1 32767\n1 0 -32768\n";
        let new = new_costs(matrix);
        let reference = reference_costs(matrix);
        assert_eq!(new, reference);
        // Spot-check the negative cost round-trip.
        assert_eq!(new[3], -1);
        // Spot-check a missing cell: (2, 0) is not listed.
        assert_eq!(new[3 + 2], 0);
    }

    #[test]
    fn test_build_gives_missing_cells_cost_zero() {
        // A 3x2 matrix that lists 2 of its 6 cells. The other 4 cost 0, as in
        // mecab-dict-index; they cost i16::MAX up to v6.2.0 (#1211).
        let input = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        std::fs::write(input.path().join("matrix.def"), "3 2\n1 0 5\n2 1 -7\n").unwrap();
        ConnectionCostMatrixBuilderOptions::default()
            .builder()
            .build(input.path(), output.path())
            .unwrap();

        let bytes = std::fs::read(output.path().join("matrix.mtx")).unwrap();
        let matrix = ConnectionCostMatrix::load(bytes).unwrap();
        let mut costs = Vec::new();
        for backward_id in 0..2 {
            for forward_id in 0..3 {
                costs.push(matrix.cost(forward_id, backward_id));
            }
        }
        assert_eq!(costs, [0, 5, 0, 0, 0, -7]);
    }

    #[test]
    fn test_no_trailing_newline() {
        let matrix = "1 1\n0 0 7";
        assert_eq!(new_costs(matrix), reference_costs(matrix));
    }

    #[test]
    fn test_duplicate_last_occurrence_wins() {
        // The reference (sequential) code keeps the last write for a duplicate
        // (forward, backward) pair; the new sequential path must match.
        let matrix = "1 1\n0 0 5\n0 0 9\n";
        let costs = new_costs(matrix);
        assert_eq!(costs[3], 9);
        assert_eq!(costs, reference_costs(matrix));
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn test_parallel_matches_sequential() {
        // Build a matrix large enough that a real build would take the
        // parallel path, and assert both paths produce identical arrays.
        let forward = 200u32;
        let backward = 200u32;
        let mut matrix = format!("{forward} {backward}\n");
        for b in 0..backward {
            for f in 0..forward {
                let cost = ((f + b) % 100) as i32 - 50;
                matrix.push_str(&format!("{f} {b} {cost}\n"));
            }
        }
        let bytes = matrix.as_bytes();
        let header_end = memchr(b'\n', bytes).unwrap();
        let data = &bytes[header_end + 1..];
        let matrix_size = MatrixSize {
            forward_size: forward,
            backward_size: backward,
        };

        let mut seq = initial_costs(matrix_size);
        fill_costs_sequential(data, 2, matrix_size, &mut seq, None).unwrap();

        let mut par = initial_costs(matrix_size);
        fill_costs_parallel(data, 2, matrix_size, &mut par, None).unwrap();

        assert_eq!(seq, par);
        assert_eq!(seq, reference_costs(&matrix));
    }

    #[test]
    fn test_missing_field_errors() {
        // A data line with only two fields is malformed (was a panic before).
        let matrix = "2 2\n0 0\n";
        let bytes = matrix.as_bytes();
        let header_end = memchr(b'\n', bytes).unwrap();
        let data = &bytes[header_end + 1..];
        let matrix_size = MatrixSize {
            forward_size: 2,
            backward_size: 2,
        };
        let mut costs = initial_costs(matrix_size);
        assert!(fill_costs_sequential(data, 2, matrix_size, &mut costs, None).is_err());
    }

    #[test]
    fn test_check_context_ids_uses_each_axis() {
        // 3 forward ids (right context ids), 2 backward ids (left context ids).
        let size = MatrixSize {
            forward_size: 3,
            backward_size: 2,
        };
        assert_eq!(size.check_context_ids(1, 2), Ok(()));
        let err = size.check_context_ids(2, 0).unwrap_err();
        assert!(err.contains("left context id 2"), "{err}");
        assert!(err.contains("matrix.def size 3 2"), "{err}");
        let err = size.check_context_ids(0, 3).unwrap_err();
        assert!(err.contains("right context id 3"), "{err}");
        // Not truncated to u16 first: 65537 would read as 1.
        assert!(size.check_context_ids(65537, 0).is_err());
    }

    #[test]
    fn test_out_of_range_entry_names_its_line() {
        // A forward id equal to the forward size used to land in the first
        // cell of the next row; it is rejected with its line number.
        let data = b"0 0 1\n\n3 0 2\n";
        let size = MatrixSize {
            forward_size: 3,
            backward_size: 2,
        };
        let mut costs = initial_costs(size);
        let err = fill_costs_sequential(data, 2, size, &mut costs, None)
            .unwrap_err()
            .to_string();
        // The header is line 1 and the blank line counts.
        assert!(err.contains("matrix.def line 4: entry (3, 0)"), "{err}");

        let err = fill_costs_sequential(b"0 -1 0\n", 2, size, &mut costs, None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("matrix.def line 2: entry (0, -1)"), "{err}");
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn test_parallel_out_of_range_entry_names_its_line() {
        // Large enough to be split into several chunks; the bad entry is far
        // from the first one, so its line number must be counted across them.
        let size = MatrixSize {
            forward_size: 200,
            backward_size: 200,
        };
        let bad_line = 30_000;
        let mut data = String::new();
        // The header is line 1, so the data region starts on line 2.
        for line in 2..40_002u32 {
            if line == bad_line {
                data.push_str("200 0 0\n");
            } else {
                data.push_str(&format!("{} {} 0\n", line % 200, line / 200 % 200));
            }
        }
        let mut costs = initial_costs(size);
        let err = fill_costs_parallel(data.as_bytes(), 2, size, &mut costs, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains(&format!("matrix.def line {bad_line}: entry (200, 0)")),
            "{err}"
        );
    }

    #[test]
    fn test_read_matrix_header_decodes_utf16() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("matrix.def");
        let text = "3 2\n0 0 0\n";

        // Labeled UTF-16BE, without a BOM.
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();
        let expected = MatrixSize {
            forward_size: 3,
            backward_size: 2,
        };
        assert_eq!(
            read_matrix_header(dir.path(), "UTF-16BE").unwrap(),
            expected
        );

        // Labeled UTF-8, with a UTF-16LE BOM, which takes precedence as in
        // the matrix stage.
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(read_matrix_header(dir.path(), "UTF-8").unwrap(), expected);

        // UTF-8 with a BOM.
        std::fs::write(&path, [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat()).unwrap();
        assert_eq!(read_matrix_header(dir.path(), "UTF-8").unwrap(), expected);
    }

    #[test]
    fn test_strip_utf8_bom() {
        let with_bom = [0xEF, 0xBB, 0xBF, b'1', b' ', b'1'];
        assert_eq!(strip_utf8_bom(&with_bom), b"1 1");
        assert_eq!(strip_utf8_bom(b"1 1"), b"1 1");
    }

    /// A 3x2 matrix with an empty line, `\r\n` line ends, a line longer than
    /// most of the block sizes tried, and a last line without a line end.
    const BLOCK_MATRIX: &str =
        "3 2\n0 0 1\r\n\n1 0 -2\n2 0                              3\n0 1 4\r\n1 1 5\n2 1 6";

    /// Fills a matrix from the data region of `matrix` read in blocks of
    /// `block_size` bytes.
    fn block_costs(matrix: &str, block_size: usize) -> LinderaResult<Vec<i16>> {
        let (header, data) = matrix.split_once('\n').unwrap();
        let matrix_size = parse_header(header.as_bytes()).unwrap();
        let mut costs = initial_costs(matrix_size);
        ConnectionCostMatrixBuilderOptions::default()
            .builder()
            .fill_costs(
                &mut io::Cursor::new(data.as_bytes()),
                block_size,
                matrix_size,
                &mut costs,
            )?;
        Ok(costs)
    }

    #[test]
    fn test_blocks_of_every_size_match_reference() {
        // Every block size from 1 byte to more than the whole text cuts the
        // lines at every position; the cut line must be carried over (#1366).
        let reference = reference_costs(BLOCK_MATRIX);
        let data_len = BLOCK_MATRIX.split_once('\n').unwrap().1.len();
        for block_size in 1..=data_len + 1 {
            let costs = block_costs(BLOCK_MATRIX, block_size)
                .unwrap_or_else(|err| panic!("block size {block_size}: {err}"));
            assert_eq!(costs, reference, "block size {block_size}");
        }
    }

    #[test]
    fn test_blocks_stay_within_their_size() {
        // With no line longer than a block, a block never holds more than its
        // size, so the file is not read whole; and every byte is handed over
        // once, in order.
        let data = "0 0 1\n1 0 2\n2 0 3\n0 1 4\n1 1 5\n2 1 6\n".repeat(20);
        let longest_line = data.lines().map(|line| line.len() + 1).max().unwrap();
        for block_size in [longest_line, longest_line + 3, 64, 200] {
            let mut seen = Vec::new();
            let mut largest = 0;
            for_each_block(
                &mut io::Cursor::new(data.as_bytes()),
                block_size,
                |block, _| {
                    largest = largest.max(block.len());
                    seen.extend_from_slice(block);
                    Ok(memchr::memchr_iter(b'\n', block).count())
                },
            )
            .unwrap();
            assert!(
                largest <= block_size,
                "block size {block_size}: a block of {largest} bytes"
            );
            assert_eq!(seen, data.as_bytes(), "block size {block_size}");
        }
    }

    #[test]
    fn test_error_line_numbers_across_blocks() {
        // The out-of-range entry (3, 0) is on line 6 of the file, whatever
        // block it falls in.
        let matrix = "3 2\n0 0 1\n\n1 0 2\n2 0 3\n3 0 4\n0 1 5\n";
        for block_size in [1, 5, 7, 12, 64] {
            let err = block_costs(matrix, block_size).unwrap_err().to_string();
            assert!(
                err.contains("matrix.def line 6: entry (3, 0)"),
                "block size {block_size}: {err}"
            );
        }
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn test_parallel_error_names_the_line_in_the_file() {
        // A block that starts on line 100 of the file: the bad entry on its
        // 30,000th line is line 30,099 of the file.
        let size = MatrixSize {
            forward_size: 200,
            backward_size: 200,
        };
        let mut data = String::new();
        for line in 0..40_000u32 {
            if line == 29_999 {
                data.push_str("200 0 0\n");
            } else {
                data.push_str(&format!("{} {} 0\n", line % 200, line / 200 % 200));
            }
        }
        let mut costs = initial_costs(size);
        let err = fill_costs_parallel(data.as_bytes(), 100, size, &mut costs, None)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("matrix.def line 30099: entry (200, 0)"),
            "{err}"
        );
    }

    /// Builds `matrix.mtx` from a `matrix.def` with the given bytes and
    /// encoding label, and returns its bytes.
    fn build_bytes(matrix_def: &[u8], encoding: &'static str) -> Vec<u8> {
        let input = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        std::fs::write(input.path().join("matrix.def"), matrix_def).unwrap();
        ConnectionCostMatrixBuilderOptions::default()
            .encoding(encoding)
            .builder()
            .build(input.path(), output.path())
            .unwrap_or_else(|err| panic!("{encoding}: {err}"));
        std::fs::read(output.path().join("matrix.mtx")).unwrap()
    }

    #[test]
    fn test_build_decodes_utf16_while_reading() {
        // The same matrix in UTF-16 (by BOM or by label) and in UTF-8 with a
        // BOM builds the same matrix.mtx as in plain UTF-8.
        let text = "3 2\n0 0 1\n1 0 -2\n2 1 7\n";
        let expected = build_bytes(text.as_bytes(), "UTF-8");

        let mut utf16le_with_bom = vec![0xFF, 0xFE];
        let mut utf16be = Vec::new();
        for unit in text.encode_utf16() {
            utf16le_with_bom.extend_from_slice(&unit.to_le_bytes());
            utf16be.extend_from_slice(&unit.to_be_bytes());
        }
        assert_eq!(build_bytes(&utf16le_with_bom, "UTF-8"), expected);
        assert_eq!(build_bytes(&utf16be, "UTF-16BE"), expected);

        let utf8_with_bom = [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat();
        assert_eq!(build_bytes(&utf8_with_bom, "UTF-8"), expected);
    }

    #[test]
    fn test_build_writes_every_piece() {
        // 300 x 300 cells are more than one piece of WRITE_CHUNK_VALUES, so
        // matrix.mtx is written in several pieces.
        let (forward, backward) = (300u32, 300u32);
        assert!((forward * backward) as usize > WRITE_CHUNK_VALUES);
        let mut matrix = format!("{forward} {backward}\n");
        for b in 0..backward {
            for f in 0..forward {
                let cost = ((f * 7 + b * 13) % 2000) as i32 - 1000;
                matrix.push_str(&format!("{f} {b} {cost}\n"));
            }
        }
        let expected: Vec<u8> = reference_costs(&matrix)
            .iter()
            .flat_map(|cost| cost.to_le_bytes())
            .collect();
        assert_eq!(build_bytes(matrix.as_bytes(), "UTF-8"), expected);
    }
}

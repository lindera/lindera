use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, Write};
#[cfg(not(target_family = "wasm"))]
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::debug;
use memchr::memchr;

use crate::LinderaResult;
use crate::dictionary::connection_cost_matrix::ConnectionCostMatrix;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::error::{LinderaError, LinderaErrorKind};
use crate::util::{read_file, write_data};

/// UTF-8 byte order mark. `encoding_rs::Encoding::decode` strips a leading
/// UTF-8 BOM, so the raw-byte fast path must strip it too to stay
/// byte-identical with the previous decode-based implementation.
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Minimum `matrix.def` data size (in bytes, excluding the header line) at
/// which the parser switches to the parallel path. Small connection matrices
/// (e.g. CC-CEDICT's 1x1, Jieba's 7x6) are parsed sequentially to avoid
/// thread-pool overhead.
#[cfg(not(target_family = "wasm"))]
const PARALLEL_THRESHOLD: usize = 1 << 20; // 1 MiB

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
    /// The parser reads the raw bytes and, for ASCII-compatible encodings,
    /// parses the space-separated integers directly without decoding the whole
    /// file into a `String`. Files whose encoding is UTF-16 (by configured
    /// label or by BOM) fall back to the decode path. On non-wasm targets a
    /// large matrix is parsed in parallel with rayon.
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Directory containing the source `matrix.def`.
    /// * `output_dir` - Directory to write `matrix.mtx` into.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a [`LinderaResult`] error if the file cannot be
    /// read, the header is missing, or a data line is malformed.
    pub fn build(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        let matrix_data_path = input_dir.join("matrix.def");
        debug!("reading {matrix_data_path:?}");
        let buffer = read_file(&matrix_data_path)?;

        // Decode only when the bytes are not ASCII-compatible (UTF-16). The
        // decoded String is kept alive here so the parser can borrow its bytes.
        let decoded = self.decode_if_needed(&buffer)?;
        let bytes: &[u8] = match &decoded {
            Some(decoded) => decoded.as_bytes(),
            None => strip_utf8_bom(&buffer),
        };

        // Parse the header line ("<forward_size> <backward_size>").
        let header_end = memchr(b'\n', bytes).unwrap_or(bytes.len());
        let matrix_size = parse_header(&bytes[..header_end])?;
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

        let len = 3 + (forward_size as usize) * (backward_size as usize);
        let mut costs = vec![i16::MAX; len];
        costs[0] = -1; // Version flag for transposed layout
        costs[1] = forward_size as i16;
        costs[2] = backward_size as i16;

        // Parse the data region (everything after the header line) and scatter
        // the costs into `costs`. Applied in file order so a duplicated
        // (forward_id, backward_id) pair keeps last-occurrence-wins semantics.
        let data = if header_end < bytes.len() {
            &bytes[header_end + 1..]
        } else {
            &[]
        };
        self.fill_costs(data, matrix_size, &mut costs)?;

        // Serialize as little-endian i16 values (identical bytes to the
        // previous per-value byteorder writes).
        let mut matrix_mtx_buffer = Vec::with_capacity(costs.len() * 2);
        for cost in &costs {
            matrix_mtx_buffer.extend_from_slice(&cost.to_le_bytes());
        }

        let wtr_matrix_mtx_path = output_dir.join(Path::new("matrix.mtx"));
        let mut wtr_matrix_mtx = io::BufWriter::new(
            File::create(wtr_matrix_mtx_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        write_data(&matrix_mtx_buffer, &mut wtr_matrix_mtx)?;
        wtr_matrix_mtx
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    /// Decode the buffer into a `String` only when the raw bytes cannot be
    /// parsed directly, i.e. when the configured encoding is UTF-16 or a
    /// UTF-16 BOM is present. `matrix.def` content is always ASCII, so every
    /// other (ASCII-compatible) encoding is parsed from the raw bytes.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The raw bytes read from `matrix.def`.
    ///
    /// # Returns
    ///
    /// `Some(decoded)` when a charset decode is required, otherwise `None`.
    fn decode_if_needed(&self, buffer: &[u8]) -> LinderaResult<Option<String>> {
        let encoding =
            Encoding::for_label_no_replacement(self.encoding.as_bytes()).ok_or_else(|| {
                LinderaErrorKind::Decode
                    .with_error(anyhow::anyhow!("Invalid encoding: {}", self.encoding))
            })?;

        let is_utf16 = encoding == UTF_16LE || encoding == UTF_16BE || has_utf16_bom(buffer);
        if is_utf16 {
            // `decode` performs BOM sniffing and honors a UTF-16 BOM over the
            // configured label, matching the previous read_file_with_encoding.
            Ok(Some(encoding.decode(buffer).0.into_owned()))
        } else {
            Ok(None)
        }
    }

    /// Parse the data region and scatter costs into `costs`.
    ///
    /// On non-wasm targets a sufficiently large region is parsed in parallel.
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes of the data region (after the header line).
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
        data: &[u8],
        matrix_size: MatrixSize,
        costs: &mut [i16],
    ) -> LinderaResult<()> {
        let remap = self.context_id_remap.as_deref();
        if data.len() >= PARALLEL_THRESHOLD {
            fill_costs_parallel(data, matrix_size, costs, remap)
        } else {
            fill_costs_sequential(data, matrix_size, costs, remap)
        }
    }

    /// Parse the data region and scatter costs into `costs` (wasm: always
    /// sequential, since rayon is unavailable on `wasm32-unknown-unknown`).
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes of the data region (after the header line).
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
        data: &[u8],
        matrix_size: MatrixSize,
        costs: &mut [i16],
    ) -> LinderaResult<()> {
        fill_costs_sequential(data, matrix_size, costs, self.context_id_remap.as_deref())
    }
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
/// * `data` - Bytes of the data region (after the header line).
/// * `line_start` - Offset of the line's first byte in `data`.
/// * `message` - What is wrong with the line.
///
/// # Returns
///
/// A `Content` error reading `matrix.def line <n>: <message>`.
fn data_line_error(data: &[u8], line_start: usize, message: String) -> LinderaError {
    // The header is line 1, so the data region starts on line 2.
    let line = 2 + memchr::memchr_iter(b'\n', &data[..line_start]).count();
    LinderaErrorKind::Content.with_error(anyhow::anyhow!("matrix.def line {line}: {message}"))
}

/// Parse the data region sequentially, scattering costs into `costs`.
///
/// # Arguments
///
/// * `data` - Bytes of the data region (after the header line).
/// * `matrix_size` - The axis sizes from the header.
/// * `costs` - Destination cost array to scatter into.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// `Ok(())`, or an error naming the line of the first malformed or
/// out-of-range entry.
fn fill_costs_sequential(
    data: &[u8],
    matrix_size: MatrixSize,
    costs: &mut [i16],
    remap: Option<&ContextIdMap>,
) -> LinderaResult<()> {
    let costs_len = costs.len();
    let mut pos = 0;
    while pos < data.len() {
        let line_end = memchr(b'\n', &data[pos..])
            .map(|offset| pos + offset)
            .unwrap_or(data.len());
        match parse_data_line(&data[pos..line_end], matrix_size, costs_len, remap) {
            Ok(Some((index, cost))) => costs[index] = cost,
            Ok(None) => {}
            Err(message) => return Err(data_line_error(data, pos, message)),
        }
        pos = line_end + 1;
    }
    Ok(())
}

/// Parse the data region in parallel, then scatter costs into `costs` in file
/// order (preserving last-occurrence-wins for duplicate entries).
///
/// The input is split into newline-aligned chunks, each parsed on a rayon
/// worker into a `(index, cost)` list; the lists are then applied in order.
///
/// # Arguments
///
/// * `data` - Bytes of the data region (after the header line).
/// * `matrix_size` - The axis sizes from the header.
/// * `costs` - Destination cost array to scatter into.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// `Ok(())`, or an error naming the line of a malformed or out-of-range
/// entry.
#[cfg(not(target_family = "wasm"))]
fn fill_costs_parallel(
    data: &[u8],
    matrix_size: MatrixSize,
    costs: &mut [i16],
    remap: Option<&ContextIdMap>,
) -> LinderaResult<()> {
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
    let partials: Vec<Vec<(usize, i16)>> = chunks
        .par_iter()
        .map(|chunk| parse_chunk(data, chunk.clone(), matrix_size, costs_len, remap))
        .collect::<LinderaResult<Vec<_>>>()?;

    for partial in &partials {
        for &(index, cost) in partial {
            costs[index] = cost;
        }
    }
    Ok(())
}

/// Parse all data lines in a single chunk into a `(index, cost)` list.
///
/// # Arguments
///
/// * `data` - Bytes of the whole data region, which the line numbers in
///   errors are counted over.
/// * `chunk` - The newline-aligned range of `data` to parse.
/// * `matrix_size` - The axis sizes from the header.
/// * `costs_len` - Length of the destination cost array, for bounds checking.
/// * `remap` - The context-id remap to relabel the ids with, if any.
///
/// # Returns
///
/// The parsed `(index, cost)` pairs in chunk order, or an error naming the
/// line of the first malformed or out-of-range entry in the chunk.
#[cfg(not(target_family = "wasm"))]
fn parse_chunk(
    data: &[u8],
    chunk: Range<usize>,
    matrix_size: MatrixSize,
    costs_len: usize,
    remap: Option<&ContextIdMap>,
) -> LinderaResult<Vec<(usize, i16)>> {
    let mut out = Vec::with_capacity(chunk.len() / 8);
    let mut pos = chunk.start;
    while pos < chunk.end {
        let line_end = memchr(b'\n', &data[pos..chunk.end])
            .map(|offset| pos + offset)
            .unwrap_or(chunk.end);
        match parse_data_line(&data[pos..line_end], matrix_size, costs_len, remap) {
            Ok(Some(entry)) => out.push(entry),
            Ok(None) => {}
            Err(message) => return Err(data_line_error(data, pos, message)),
        }
        pos = line_end + 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference parser replicating the previous split_whitespace + from_str
    /// implementation, used to assert byte-identical output.
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
        let mut costs = vec![i16::MAX; len];
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
        let len = 3 + (forward_size as usize) * (backward_size as usize);
        let mut costs = vec![i16::MAX; len];
        costs[0] = -1;
        costs[1] = forward_size as i16;
        costs[2] = backward_size as i16;
        let data = if header_end < bytes.len() {
            &bytes[header_end + 1..]
        } else {
            &[]
        };
        fill_costs_sequential(data, matrix_size, &mut costs, None).unwrap();
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
        // Missing cells default to i16::MAX; negative cost round-trips via the
        // u16 cast; multiple spaces between fields.
        let matrix = "3 2\n0  0  -1\n2 1 32767\n1 0 -32768\n";
        let new = new_costs(matrix);
        let reference = reference_costs(matrix);
        assert_eq!(new, reference);
        // Spot-check the negative cost round-trip.
        assert_eq!(new[3], -1);
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
        let len = 3 + (forward as usize) * (backward as usize);
        let matrix_size = MatrixSize {
            forward_size: forward,
            backward_size: backward,
        };

        let mut seq = vec![i16::MAX; len];
        seq[0] = -1;
        seq[1] = forward as i16;
        seq[2] = backward as i16;
        fill_costs_sequential(data, matrix_size, &mut seq, None).unwrap();

        let mut par = vec![i16::MAX; len];
        par[0] = -1;
        par[1] = forward as i16;
        par[2] = backward as i16;
        fill_costs_parallel(data, matrix_size, &mut par, None).unwrap();

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
        let mut costs = vec![i16::MAX; 3 + 4];
        let matrix_size = MatrixSize {
            forward_size: 2,
            backward_size: 2,
        };
        assert!(fill_costs_sequential(data, matrix_size, &mut costs, None).is_err());
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
        let mut costs = vec![i16::MAX; 3 + 6];
        let err = fill_costs_sequential(data, size, &mut costs, None)
            .unwrap_err()
            .to_string();
        // The header is line 1 and the blank line counts.
        assert!(err.contains("matrix.def line 4: entry (3, 0)"), "{err}");

        let err = fill_costs_sequential(b"0 -1 0\n", size, &mut costs, None)
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
        let mut costs = vec![i16::MAX; 3 + 200 * 200];
        let err = fill_costs_parallel(data.as_bytes(), size, &mut costs, None)
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
}

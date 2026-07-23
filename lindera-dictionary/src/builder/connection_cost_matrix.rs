use std::borrow::Cow;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use derive_builder::Builder;
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use log::debug;
use memchr::memchr;

use crate::LinderaResult;
use crate::dictionary::context_id_map::ContextIdMap;
use crate::error::LinderaErrorKind;
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

/// Builder for the connection cost matrix (`matrix.mtx`).
#[derive(Builder, Debug)]
#[builder(name = ConnectionCostMatrixBuilderOptions)]
#[builder(build_fn(name = "builder"))]
pub struct ConnectionCostMatrixBuilder {
    /// Character encoding of the source `matrix.def` file.
    ///
    /// If set to UTF-8, files with a UTF-16 BOM are still decoded correctly.
    #[builder(default = "\"UTF-8\".into()", setter(into))]
    encoding: Cow<'static, str>,
    /// Optional connection-cost context-ID remapping. When present, `forward_id`
    /// (right-context id) is mapped through `remap.right` and `backward_id`
    /// (left-context id) through `remap.left` before the cost is scattered, so
    /// frequently-used cells cluster near the front of each row. `None` keeps the
    /// output byte-identical to the un-remapped build.
    #[builder(default = "None")]
    context_id_remap: Option<Arc<ContextIdMap>>,
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
        let mut header_pos = 0;
        let forward_size = next_int(&bytes[..header_end], &mut header_pos).ok_or_else(|| {
            LinderaErrorKind::Content
                .with_error(anyhow::anyhow!("matrix.def is missing the size header"))
        })? as u32;
        let backward_size = next_int(&bytes[..header_end], &mut header_pos).ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                "matrix.def header is missing backward size"
            ))
        })? as u32;

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
        self.fill_costs(data, forward_size, &mut costs)?;

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
    /// * `forward_size` - Number of forward context IDs (matrix stride).
    /// * `costs` - Destination cost array to scatter into.
    #[cfg(not(target_family = "wasm"))]
    fn fill_costs(&self, data: &[u8], forward_size: u32, costs: &mut [i16]) -> LinderaResult<()> {
        let remap = self.context_id_remap.as_deref();
        if data.len() >= PARALLEL_THRESHOLD {
            fill_costs_parallel(data, forward_size, costs, remap)
        } else {
            fill_costs_sequential(data, forward_size, costs, remap)
        }
    }

    /// Parse the data region and scatter costs into `costs` (wasm: always
    /// sequential, since rayon is unavailable on `wasm32-unknown-unknown`).
    ///
    /// # Arguments
    ///
    /// * `data` - Bytes of the data region (after the header line).
    /// * `forward_size` - Number of forward context IDs (matrix stride).
    /// * `costs` - Destination cost array to scatter into.
    #[cfg(target_family = "wasm")]
    fn fill_costs(&self, data: &[u8], forward_size: u32, costs: &mut [i16]) -> LinderaResult<()> {
        fill_costs_sequential(data, forward_size, costs, self.context_id_remap.as_deref())
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
/// loading the whole (potentially huge) matrix body. `matrix.def` is ASCII in every
/// shipped dictionary, so the first line is parsed directly; `encoding` is accepted
/// for signature symmetry with the rest of the builder and is currently unused.
///
/// This is the single source of truth for the connection-matrix axis sizes used by
/// [`super::context_id_remap::compute_context_id_remap`], so the remap permutations
/// are always sized to the same axes the matrix build scatters into.
///
/// # Arguments
///
/// * `input_dir` - Directory containing `matrix.def`.
/// * `_encoding` - Source encoding label (unused; `matrix.def` is ASCII).
///
/// # Returns
///
/// `(forward_size, backward_size)`, or an error if the file is missing or the header
/// is malformed.
pub(crate) fn read_matrix_header(input_dir: &Path, _encoding: &str) -> LinderaResult<(u32, u32)> {
    let path = input_dir.join("matrix.def");
    let file = File::open(&path).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context(format!("Failed to open matrix.def: {path:?}"))
    })?;
    let mut reader = io::BufReader::new(file);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).map_err(|err| {
        LinderaErrorKind::Io
            .with_error(anyhow::anyhow!(err))
            .add_context("Failed to read matrix.def header line")
    })?;
    let bytes = strip_utf8_bom(&line);
    let mut pos = 0;
    let forward_size = next_int(bytes, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content
            .with_error(anyhow::anyhow!("matrix.def is missing the size header"))
    })? as u32;
    let backward_size = next_int(bytes, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "matrix.def header is missing backward size"
        ))
    })? as u32;
    Ok((forward_size, backward_size))
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
/// Casts match the previous implementation exactly (`forward_id`/`backward_id`
/// via `i32 as u32`, `cost` via `i32 as u16 as i16`) so the resulting bytes
/// are identical.
///
/// # Arguments
///
/// * `line` - The line bytes (without the trailing newline).
/// * `forward_size` - Number of forward context IDs (matrix stride).
/// * `costs_len` - Length of the destination cost array, for bounds checking.
///
/// # Returns
///
/// `Ok(Some((index, cost)))` for a data line, `Ok(None)` for an empty or
/// whitespace-only line, or an error for a malformed or out-of-range line.
fn parse_data_line(
    line: &[u8],
    forward_size: u32,
    costs_len: usize,
    remap: Option<&ContextIdMap>,
) -> LinderaResult<Option<(usize, i16)>> {
    let mut pos = 0;
    let Some(forward_id) = next_int(line, &mut pos) else {
        // Empty or whitespace-only line: skip it.
        return Ok(None);
    };
    let backward_id = next_int(line, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content
            .with_error(anyhow::anyhow!("matrix.def line is missing backward id"))
    })?;
    let cost = next_int(line, &mut pos).ok_or_else(|| {
        LinderaErrorKind::Content.with_error(anyhow::anyhow!("matrix.def line is missing cost"))
    })?;

    let forward_id = forward_id as u32 as usize;
    let backward_id = backward_id as u32 as usize;
    // Apply the frequency remap (right-context id via P_right, left-context id via
    // P_left). Sizes are guaranteed equal to the axes by the guard in `build`, so an
    // out-of-range source id here is a malformed matrix.def, reported like the
    // index check below.
    let (fwd, bwd) = match remap {
        Some(m) => {
            if forward_id >= m.right.len() || backward_id >= m.left.len() {
                return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
                    "matrix.def entry ({forward_id}, {backward_id}) is out of range"
                )));
            }
            (m.right[forward_id] as usize, m.left[backward_id] as usize)
        }
        None => (forward_id, backward_id),
    };
    let index = 3 + fwd + bwd * forward_size as usize;
    if index >= costs_len {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "matrix.def entry ({forward_id}, {backward_id}) is out of range"
        )));
    }
    let cost = (cost as u16) as i16;
    Ok(Some((index, cost)))
}

/// Parse the data region sequentially, scattering costs into `costs`.
///
/// # Arguments
///
/// * `data` - Bytes of the data region (after the header line).
/// * `forward_size` - Number of forward context IDs (matrix stride).
/// * `costs` - Destination cost array to scatter into.
fn fill_costs_sequential(
    data: &[u8],
    forward_size: u32,
    costs: &mut [i16],
    remap: Option<&ContextIdMap>,
) -> LinderaResult<()> {
    let costs_len = costs.len();
    let mut pos = 0;
    while pos < data.len() {
        let line_end = memchr(b'\n', &data[pos..])
            .map(|offset| pos + offset)
            .unwrap_or(data.len());
        if let Some((index, cost)) =
            parse_data_line(&data[pos..line_end], forward_size, costs_len, remap)?
        {
            costs[index] = cost;
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
/// * `forward_size` - Number of forward context IDs (matrix stride).
/// * `costs` - Destination cost array to scatter into.
#[cfg(not(target_family = "wasm"))]
fn fill_costs_parallel(
    data: &[u8],
    forward_size: u32,
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

    let chunks: Vec<&[u8]> = bounds.windows(2).map(|w| &data[w[0]..w[1]]).collect();
    let partials: Vec<Vec<(usize, i16)>> = chunks
        .par_iter()
        .map(|chunk| parse_chunk(chunk, forward_size, costs_len, remap))
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
/// * `chunk` - A newline-aligned slice of the data region.
/// * `forward_size` - Number of forward context IDs (matrix stride).
/// * `costs_len` - Length of the destination cost array, for bounds checking.
///
/// # Returns
///
/// The parsed `(index, cost)` pairs in chunk order.
#[cfg(not(target_family = "wasm"))]
fn parse_chunk(
    chunk: &[u8],
    forward_size: u32,
    costs_len: usize,
    remap: Option<&ContextIdMap>,
) -> LinderaResult<Vec<(usize, i16)>> {
    let mut out = Vec::with_capacity(chunk.len() / 8);
    let mut pos = 0;
    while pos < chunk.len() {
        let line_end = memchr(b'\n', &chunk[pos..])
            .map(|offset| pos + offset)
            .unwrap_or(chunk.len());
        if let Some(entry) = parse_data_line(&chunk[pos..line_end], forward_size, costs_len, remap)?
        {
            out.push(entry);
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
        fill_costs_sequential(data, forward_size, &mut costs, None).unwrap();
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

        let mut seq = vec![i16::MAX; len];
        seq[0] = -1;
        seq[1] = forward as i16;
        seq[2] = backward as i16;
        fill_costs_sequential(data, forward, &mut seq, None).unwrap();

        let mut par = vec![i16::MAX; len];
        par[0] = -1;
        par[1] = forward as i16;
        par[2] = backward as i16;
        fill_costs_parallel(data, forward, &mut par, None).unwrap();

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
        assert!(fill_costs_sequential(data, 2, &mut costs, None).is_err());
    }

    #[test]
    fn test_strip_utf8_bom() {
        let with_bom = [0xEF, 0xBB, 0xBF, b'1', b' ', b'1'];
        assert_eq!(strip_utf8_bom(&with_bom), b"1 1");
        assert_eq!(strip_utf8_bom(b"1 1"), b"1 1");
    }
}

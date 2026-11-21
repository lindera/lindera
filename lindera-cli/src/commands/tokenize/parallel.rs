//! `lindera tokenize` on several threads (`--max-threads` of 2 or more).
//!
//! A reader thread cuts the input into chunks of whole lines, worker threads
//! tokenize the lines of the chunks and format their output, and the calling
//! thread writes the output of the chunks in input order. Every line is
//! tokenized as a call of its own by a worker of the same tokenizer, and
//! formatted by the same functions as on one thread, so the output is the
//! same, byte for byte.
//!
//! The reader and the workers are not joined. With an input that stays
//! open, the reader waits in a read that does not return; the calling
//! thread still writes what is ready, and when it stops, on an error or on a
//! standard output without a reader, the process ends with the threads in
//! it, without reading the rest of the input, as on one thread.

use std::any::Any;
use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use lindera::LinderaResult;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::analysis::worker::AnalysisWorker;
use lindera::error::{LinderaError, LinderaErrorKind};

use super::{Format, strip_line_terminator, write_nbest_output, write_output};
use crate::commands::{io_err, stdout_err};

/// The input, in bytes, after which a chunk is cut. Smaller chunks balance
/// the threads better and larger ones cost fewer messages. Affects only the
/// speed.
const CHUNK_BYTES: usize = 64 * 1024;

/// The lines after which a chunk is cut, for an input of many short lines.
/// Affects only the speed.
const CHUNK_LINES: usize = 4096;

/// The capacity of the reader's buffer. A chunk is also cut where the buffer
/// holds no complete line, since reading on may wait for more input, so the
/// buffer holds several chunks. Affects only the speed.
const READ_CAPACITY: usize = 1024 * 1024;

/// The chunks read but not yet written, per worker thread: a bound on the
/// memory that the input and the output in flight take.
const CHUNKS_IN_FLIGHT_PER_THREAD: usize = 2;

/// How every line is tokenized and written.
#[derive(Clone, Copy)]
pub(super) struct Settings {
    /// The output format.
    pub(super) format: Format,
    /// The number of N-best results; 1 or less tokenizes the best path only.
    pub(super) nbest: usize,
    /// Deduplicate N-best results with identical word boundaries.
    pub(super) nbest_unique: bool,
    /// Discard N-best results costing more than the best one + threshold.
    pub(super) nbest_cost_threshold: Option<i64>,
}

/// Lines of the input, read together.
struct Chunk {
    /// The place of the chunk in the input.
    seq: usize,
    /// The lines, with their terminators.
    text: String,
    /// The end of every line in `text`.
    ends: Vec<usize>,
}

/// What became of a chunk, or of the input after its last chunk.
enum Outcome {
    /// The output of every line of the chunk.
    Written(Vec<u8>),
    /// The output of the lines before a line whose tokenization failed, and
    /// the error.
    Failed(Vec<u8>, LinderaError),
    /// The output of the lines before a line whose tokenization panicked,
    /// and the panic.
    Panicked(Vec<u8>, Box<dyn Any + Send>),
    /// The error of a read, after the chunks of the lines before it.
    ReadFailed(io::Error),
    /// The end of the input, after its last chunk.
    End,
}

/// Tokenizes every line of `input` on `threads` threads and writes the
/// output to the standard output in input order.
///
/// It is not inlined into [`super::tokenize`], so that it does not change
/// how the loop on one thread there is compiled.
///
/// # Arguments
///
/// * `tokenizer` - The tokenizer; every thread makes a worker from it.
/// * `input` - The input file or the standard input.
/// * `settings` - How every line is tokenized and written.
/// * `threads` - The number of threads that tokenize.
///
/// # Returns
///
/// `Ok(())` once the output is written; the error of the first line in
/// input order whose tokenization failed, of a read, or of a write; the
/// panic of the first line that panicked is resumed.
#[inline(never)]
pub(super) fn tokenize_lines(
    tokenizer: Tokenizer,
    input: Box<dyn Read + Send>,
    settings: Settings,
    threads: usize,
) -> LinderaResult<()> {
    let tokenizer = Arc::new(tokenizer);
    let in_flight = threads.saturating_mul(CHUNKS_IN_FLIGHT_PER_THREAD);
    let (credit_tx, credit_rx) = mpsc::sync_channel::<()>(in_flight);
    for _ in 0..in_flight {
        // The channel has room for all of them.
        let _ = credit_tx.try_send(());
    }
    let (chunk_tx, chunk_rx) = mpsc::channel::<Chunk>();
    let chunk_rx = Arc::new(Mutex::new(chunk_rx));
    let (outcome_tx, outcome_rx) = mpsc::channel::<(usize, Outcome)>();

    for index in 0..threads {
        let tokenizer = Arc::clone(&tokenizer);
        let chunks = Arc::clone(&chunk_rx);
        let outcomes = outcome_tx.clone();
        thread::Builder::new()
            .name(format!("lindera-tokenize-{}", index + 1))
            .spawn(move || work(&tokenizer, &chunks, &outcomes, settings))
            .map_err(io_err)?;
    }
    // The reader takes the last sender of the outcomes, so that the writer
    // sees the channel close if every other thread ends without the end of
    // the input.
    thread::Builder::new()
        .name("lindera-read".to_string())
        .spawn(move || read(input, &chunk_tx, &outcome_tx, &credit_rx))
        .map_err(io_err)?;

    // Buffer all output on a locked stdout, as on one thread. On an error,
    // the writer is dropped with what it holds, and flushed as on one
    // thread.
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    write_in_order(&mut writer, &outcome_rx, &credit_tx)
}

/// Reads `input` into chunks of whole lines and sends them to the workers,
/// then sends the end of the input, or the error of a read, to the writer.
///
/// # Arguments
///
/// * `input` - The input.
/// * `chunks` - Where the chunks go, to the workers.
/// * `outcomes` - Where the end of the input or a read error goes, to the
///   writer.
/// * `credits` - One credit per chunk read, given back as the writer writes
///   chunks; closed once the writer has stopped.
fn read(
    input: Box<dyn Read + Send>,
    chunks: &Sender<Chunk>,
    outcomes: &Sender<(usize, Outcome)>,
    credits: &Receiver<()>,
) {
    let mut reader = BufReader::with_capacity(READ_CAPACITY, input);
    let mut seq = 0;
    loop {
        if credits.recv().is_err() {
            return;
        }
        let mut text = String::new();
        let mut ends = Vec::new();
        let last = loop {
            match reader.read_line(&mut text) {
                Ok(0) => break Some(Outcome::End),
                Ok(_) => ends.push(text.len()),
                Err(err) => {
                    // Keep the whole lines read before the error.
                    text.truncate(ends.last().copied().unwrap_or(0));
                    break Some(Outcome::ReadFailed(err));
                }
            }
            // Reading on may wait for more input, so the chunk is cut where
            // the buffer holds no complete line: what was read gets written
            // meanwhile.
            if text.len() >= CHUNK_BYTES
                || ends.len() >= CHUNK_LINES
                || !reader.buffer().contains(&b'\n')
            {
                break None;
            }
        };
        if !ends.is_empty() {
            if chunks.send(Chunk { seq, text, ends }).is_err() {
                return;
            }
            seq += 1;
        }
        if let Some(outcome) = last {
            // The writer may have stopped already.
            let _ = outcomes.send((seq, outcome));
            return;
        }
    }
}

/// Tokenizes the chunks that come in, with a worker of `tokenizer`, until
/// none is left, and sends their outcomes to the writer.
///
/// # Arguments
///
/// * `tokenizer` - The tokenizer to make the thread's worker from.
/// * `chunks` - The chunks, shared by the workers.
/// * `outcomes` - Where the outcomes go, to the writer.
/// * `settings` - How every line is tokenized and written.
fn work(
    tokenizer: &Tokenizer,
    chunks: &Mutex<Receiver<Chunk>>,
    outcomes: &Sender<(usize, Outcome)>,
    settings: Settings,
) {
    let mut worker = tokenizer.new_worker();
    let mut details_buf = String::new();
    loop {
        // Nothing panics while the lock is held, and a poisoned lock still
        // holds a working receiver.
        let received = chunks.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let Ok(chunk) = received else {
            return;
        };
        let outcome = tokenize_chunk(&mut worker, &chunk, settings, &mut details_buf);
        if outcomes.send((chunk.seq, outcome)).is_err() {
            return;
        }
    }
}

/// Tokenizes the lines of a chunk and formats their output, stopping at the
/// first line that fails or panics.
///
/// # Arguments
///
/// * `worker` - The thread's worker.
/// * `chunk` - The chunk.
/// * `settings` - How every line is tokenized and written.
/// * `details_buf` - A scratch buffer reused across tokens for joining
///   detail fields.
///
/// # Returns
///
/// The output of the chunk, or of its lines before the one that failed or
/// panicked, with the error or the panic.
fn tokenize_chunk(
    worker: &mut AnalysisWorker,
    chunk: &Chunk,
    settings: Settings,
    details_buf: &mut String,
) -> Outcome {
    let mut output = Vec::with_capacity(chunk.text.len() * 2);
    let mut start = 0;
    for &end in &chunk.ends {
        // The ends are the ends of the lines `read_line` read, at character
        // boundaries.
        let Some(line) = chunk.text.get(start..end) else {
            break;
        };
        start = end;
        let line = strip_line_terminator(line);
        let written = output.len();
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            tokenize_line(worker, line, settings, &mut output, details_buf)
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                output.truncate(written);
                return Outcome::Failed(output, err);
            }
            Err(payload) => {
                // The panic may have left the worker's buffers in an
                // intermediate state, and the line's output unfinished.
                worker.reset();
                output.truncate(written);
                return Outcome::Panicked(output, payload);
            }
        }
    }
    Outcome::Written(output)
}

/// Tokenizes one line and appends its output, as the loop on one thread
/// writes it.
///
/// # Arguments
///
/// * `worker` - The thread's worker.
/// * `line` - The line, without its terminator.
/// * `settings` - How the line is tokenized and written.
/// * `output` - The output of the chunk.
/// * `details_buf` - A scratch buffer reused across tokens for joining
///   detail fields.
///
/// # Returns
///
/// `Ok(())`, or the error of the tokenization.
fn tokenize_line(
    worker: &mut AnalysisWorker,
    line: &str,
    settings: Settings,
    output: &mut Vec<u8>,
    details_buf: &mut String,
) -> LinderaResult<()> {
    // Writing to a `Vec` does not fail.
    if settings.nbest >= 2 {
        let results = worker.tokenize_nbest(
            line,
            settings.nbest,
            settings.nbest_unique,
            settings.nbest_cost_threshold,
        )?;
        write_nbest_output(output, settings.format, results, details_buf).map_err(io_err)
    } else {
        let tokens = worker.tokenize(line)?;
        write_output(output, settings.format, tokens, details_buf).map_err(io_err)
    }
}

/// Writes the output of the chunks to `writer` in input order, until the end
/// of the input or the first failure.
///
/// # Arguments
///
/// * `writer` - The standard output.
/// * `outcomes` - The outcomes of the chunks, in any order, and the end of
///   the input.
/// * `credits` - Where a credit goes back to the reader for every chunk
///   written.
///
/// # Returns
///
/// `Ok(())` at the end of the input, once the output is flushed, or the
/// error of the first failure in input order, with the output before it
/// written but not flushed; the first panic is resumed.
fn write_in_order<W: Write>(
    writer: &mut W,
    outcomes: &Receiver<(usize, Outcome)>,
    credits: &SyncSender<()>,
) -> LinderaResult<()> {
    let mut ready = BTreeMap::new();
    let mut next = 0;
    loop {
        while let Some(outcome) = ready.remove(&next) {
            next += 1;
            match outcome {
                Outcome::Written(output) => {
                    writer.write_all(&output).map_err(stdout_err)?;
                    // A full channel holds enough credits already, and a
                    // closed one has no reader left.
                    let _ = credits.try_send(());
                }
                // As on one thread, the output before the failing line is
                // flushed when the writer is dropped, and an error of that
                // flush is ignored.
                Outcome::Failed(output, err) => {
                    writer.write_all(&output).map_err(stdout_err)?;
                    return Err(err);
                }
                Outcome::Panicked(output, payload) => {
                    let _ = writer.write_all(&output);
                    panic::resume_unwind(payload);
                }
                Outcome::ReadFailed(err) => return Err(io_err(err)),
                Outcome::End => {
                    // Surface write errors instead of letting the implicit
                    // flush on drop swallow them.
                    writer.flush().map_err(stdout_err)?;
                    return Ok(());
                }
            }
        }
        match outcomes.recv() {
            Ok((seq, outcome)) => {
                ready.insert(seq, outcome);
            }
            Err(_) => {
                return Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
                    "the tokenizing threads ended before the end of the input"
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Cursor, Read};
    use std::sync::mpsc;

    use lindera::error::LinderaErrorKind;

    use super::{CHUNK_LINES, Chunk, Outcome, read, write_in_order};

    /// What `read` sent: the lines of every chunk with its place, and the
    /// outcome that ended the input with its place.
    struct ReadOutput {
        /// The place of every chunk, and its lines with their terminators.
        chunks: Vec<(usize, Vec<String>)>,
        /// The place and the kind of the outcome that ended the input.
        last: Vec<(usize, &'static str)>,
    }

    /// Runs `read` on `input` with enough credits for every chunk, and
    /// collects what it sent.
    fn read_all(input: Box<dyn Read + Send>) -> ReadOutput {
        let (credit_tx, credit_rx) = mpsc::sync_channel(100_000);
        for _ in 0..100_000 {
            credit_tx.try_send(()).unwrap();
        }
        let (chunk_tx, chunk_rx) = mpsc::channel::<Chunk>();
        let (outcome_tx, outcome_rx) = mpsc::channel();
        read(input, &chunk_tx, &outcome_tx, &credit_rx);
        drop((chunk_tx, outcome_tx));
        let chunks = chunk_rx
            .iter()
            .map(|chunk| {
                let mut start = 0;
                let lines = chunk
                    .ends
                    .iter()
                    .map(|&end| {
                        let line = chunk.text[start..end].to_string();
                        start = end;
                        line
                    })
                    .collect();
                (chunk.seq, lines)
            })
            .collect();
        let last = outcome_rx
            .iter()
            .map(|(seq, outcome)| {
                let kind = match outcome {
                    Outcome::End => "end",
                    Outcome::ReadFailed(_) => "read failed",
                    _ => "other",
                };
                (seq, kind)
            })
            .collect();
        ReadOutput { chunks, last }
    }

    /// An input that gives one line per read, as a terminal or a pipe fed
    /// line by line does: a read after the last line given would wait.
    struct LineByLine(VecDeque<Vec<u8>>);

    impl Read for LineByLine {
        /// Gives the next line, or the end of the input once none is left.
        ///
        /// # Arguments
        ///
        /// * `buf` - The reader's buffer, larger than every line.
        ///
        /// # Returns
        ///
        /// The length of the line given, or 0 at the end.
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.0.pop_front() {
                Some(line) => {
                    buf[..line.len()].copy_from_slice(&line);
                    Ok(line.len())
                }
                None => Ok(0),
            }
        }
    }

    /// An input in the reader's buffer at once is cut every
    /// `CHUNK_LINES` lines.
    #[test]
    fn chunks_are_cut_at_the_line_limit() {
        let input = "a\n".repeat(2 * CHUNK_LINES + 10);
        let read = read_all(Box::new(Cursor::new(input)));
        let sizes: Vec<(usize, usize)> = read
            .chunks
            .iter()
            .map(|(seq, lines)| (*seq, lines.len()))
            .collect();
        assert_eq!(sizes, [(0, CHUNK_LINES), (1, CHUNK_LINES), (2, 10)]);
        assert_eq!(read.last, [(3, "end")]);
    }

    /// Reading on would wait, so every line given by a read is a chunk of
    /// its own, sent before the next read.
    #[test]
    fn chunks_are_cut_where_reading_on_may_wait() {
        let input = LineByLine(
            ["東京\n", "大阪\n", "京都\n"]
                .iter()
                .map(|line| line.as_bytes().to_vec())
                .collect(),
        );
        let read = read_all(Box::new(input));
        assert_eq!(
            read.chunks,
            [
                (0, vec!["東京\n".to_string()]),
                (1, vec!["大阪\n".to_string()]),
                (2, vec!["京都\n".to_string()]),
            ]
        );
        assert_eq!(read.last, [(3, "end")]);
    }

    /// A last line without a terminator is read. The buffer holds no
    /// complete line after the first one, so the first one is a chunk of
    /// its own.
    #[test]
    fn a_last_line_without_a_terminator_is_read() {
        let read = read_all(Box::new(Cursor::new("東京\r\n大阪")));
        assert_eq!(
            read.chunks,
            [
                (0, vec!["東京\r\n".to_string()]),
                (1, vec!["大阪".to_string()]),
            ]
        );
        assert_eq!(read.last, [(2, "end")]);
    }

    /// The lines before an invalid line are sent, and the read error comes
    /// after them.
    #[test]
    fn a_read_error_comes_after_the_lines_before_it() {
        let read = read_all(Box::new(Cursor::new(
            b"\xe6\x9d\xb1\n\xff\n\xe5\xa4\xa7\n".to_vec(),
        )));
        assert_eq!(read.chunks, [(0, vec!["東\n".to_string()])]);
        assert_eq!(read.last, [(1, "read failed")]);
    }

    /// An empty input sends only its end.
    #[test]
    fn an_empty_input_has_no_chunk() {
        let read = read_all(Box::new(Cursor::new("")));
        assert!(read.chunks.is_empty());
        assert_eq!(read.last, [(0, "end")]);
    }

    /// Sends `outcomes` in the given order, then runs `write_in_order` on
    /// them, and returns what it wrote, what it returned, and the number of
    /// credits it gave back.
    fn write_all(outcomes: Vec<(usize, Outcome)>) -> (String, Result<(), String>, usize) {
        let (outcome_tx, outcome_rx) = mpsc::channel();
        for outcome in outcomes {
            outcome_tx.send(outcome).unwrap();
        }
        drop(outcome_tx);
        let (credit_tx, credit_rx) = mpsc::sync_channel(100);
        let mut output = Vec::new();
        let result =
            write_in_order(&mut output, &outcome_rx, &credit_tx).map_err(|err| err.to_string());
        drop(credit_tx);
        let credits = credit_rx.iter().count();
        (String::from_utf8(output).unwrap(), result, credits)
    }

    /// The outcome of a chunk whose output is `text`.
    fn written(text: &str) -> Outcome {
        Outcome::Written(text.as_bytes().to_vec())
    }

    /// Chunks that come in out of order are written in input order, and a
    /// credit goes back for each.
    #[test]
    fn chunks_are_written_in_input_order() {
        let (output, result, credits) = write_all(vec![
            (2, written("c")),
            (3, Outcome::End),
            (0, written("a")),
            (1, written("b")),
        ]);
        assert_eq!(output, "abc");
        assert_eq!(result, Ok(()));
        assert_eq!(credits, 3, "one credit per chunk written");
    }

    /// A failed line ends the output after the chunks before it and the
    /// lines of its chunk before it; the chunks after it are not written.
    #[test]
    fn the_first_failure_in_input_order_ends_the_output() {
        let error = LinderaErrorKind::Content.with_error(anyhow::anyhow!("the line failed"));
        let (output, result, _) = write_all(vec![
            (2, written("c")),
            (1, Outcome::Failed(b"b".to_vec(), error)),
            (3, Outcome::End),
            (0, written("a")),
        ]);
        assert_eq!(output, "ab");
        match result {
            Ok(()) => panic!("the failure was not returned"),
            Err(err) => assert!(err.contains("the line failed"), "got: {err}"),
        }
    }

    /// A read error is returned after the output of the chunks before it.
    #[test]
    fn a_read_error_comes_after_the_chunks_before_it() {
        let error = io::Error::new(io::ErrorKind::InvalidData, "not UTF-8");
        let (output, result, _) =
            write_all(vec![(1, Outcome::ReadFailed(error)), (0, written("a"))]);
        assert_eq!(output, "a");
        match result {
            Ok(()) => panic!("the read error was not returned"),
            Err(err) => assert!(err.contains("not UTF-8"), "got: {err}"),
        }
    }

    /// The panic of a line is resumed after the output before it.
    #[test]
    fn a_panic_is_resumed_after_the_output_before_it() {
        let (outcome_tx, outcome_rx) = mpsc::channel();
        outcome_tx
            .send((
                1,
                Outcome::Panicked(b"b".to_vec(), Box::new("the line panicked")),
            ))
            .unwrap();
        outcome_tx.send((0, written("a"))).unwrap();
        let (credit_tx, _credit_rx) = mpsc::sync_channel(100);
        let mut output = Vec::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            write_in_order(&mut output, &outcome_rx, &credit_tx)
        }));
        match result {
            Ok(_) => panic!("the panic was not resumed"),
            Err(payload) => assert_eq!(payload.downcast_ref::<&str>(), Some(&"the line panicked")),
        }
        assert_eq!(output, b"ab");
    }

    /// Without the end of the input, the writer stops once every thread
    /// has ended, instead of waiting forever.
    #[test]
    fn the_writer_stops_when_the_threads_end_early() {
        let (output, result, _) = write_all(vec![(0, written("a"))]);
        assert_eq!(output, "a");
        assert!(result.is_err());
    }
}

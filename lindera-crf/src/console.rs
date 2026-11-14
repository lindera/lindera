//! Writing the training's progress lines to the standard output and the
//! standard error without panicking.
//!
//! The training runs inside other programs: the CLI, and the Node.js,
//! Python, Ruby and PHP bindings. Their standard streams may be non-blocking
//! (Node.js makes them so when they are pipes, and `node --test` does for
//! every test file), or may have no reader any more. `println!` and
//! `eprintln!` panic when a write fails, and a panic on the thread that
//! called a Node.js function aborts the whole process (#1312).
//!
//! [`write_line`] writes as a write to a blocking stream would: it waits
//! while the stream is full, so that no line is lost or cut, and it drops a
//! line that the stream can no longer take, so that the training goes on.
//! Progress is a side output: whatever happens to it, the training gives the
//! same result.

use core::fmt;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// The first wait after a write that would block.
///
/// A full stream is drained by another process, which may take a while,
/// while a short first wait keeps the delay small when it drains at once.
const WAIT_FIRST: Duration = Duration::from_millis(1);

/// The longest wait between two tries of a write that would block.
///
/// The wait doubles from [`WAIT_FIRST`] up to this, so that a stream that
/// stays full for long costs few tries, while the line is written at most
/// this long after the stream has room again.
const WAIT_MAX: Duration = Duration::from_millis(50);

/// A standard stream to write a line to.
#[cfg_attr(docsrs, doc(cfg(feature = "console")))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    /// The standard output.
    Stdout,
    /// The standard error.
    Stderr,
}

/// Writes a line, and a newline after it, to a standard stream.
///
/// The line is formatted first and written with the stream's lock held, so
/// that lines written from several threads do not mix. A write that would
/// block, because the stream is non-blocking and full, is tried again after
/// a wait of 1 ms, doubling up to 50 ms, for as long as it takes; an
/// interrupted write at once. Any other error, such as a broken pipe, drops
/// the rest of the line. Nothing is returned, so that a line that could not
/// be written cannot fail the caller.
///
/// # Arguments
///
/// * `stream` - The stream to write to.
/// * `args` - The line, without the newline, as `format_args!` makes it.
#[cfg_attr(docsrs, doc(cfg(feature = "console")))]
pub fn write_line(stream: Stream, args: fmt::Arguments<'_>) {
    let mut line = alloc::fmt::format(args);
    line.push('\n');
    let mut sleep = thread::sleep;
    // The line is dropped on an error, as described above.
    let _ = match stream {
        Stream::Stdout => write_line_to(&mut io::stdout().lock(), line.as_bytes(), &mut sleep),
        Stream::Stderr => write_line_to(&mut io::stderr().lock(), line.as_bytes(), &mut sleep),
    };
}

/// Writes `line` to `writer` and flushes it, waiting while it would block.
///
/// `write_all` is not used, since after an error it does not tell how much
/// was written, and trying the whole line again would write its start
/// twice. The flush is needed because Rust's standard output keeps the rest
/// of a line that was written only in part in its buffer, and reports it as
/// written.
///
/// # Arguments
///
/// * `writer` - The writer.
/// * `line` - The bytes to write.
/// * `wait` - Called with the time to wait before trying a write that would
///   block again.
///
/// # Returns
///
/// `Ok(())` once every byte is written and flushed, or the error that ended
/// the write: [`io::ErrorKind::WriteZero`] when the writer took no byte, or
/// the first error other than an interrupted write or one that would block.
fn write_line_to<W: Write + ?Sized>(
    writer: &mut W,
    line: &[u8],
    wait: &mut dyn FnMut(Duration),
) -> io::Result<()> {
    let mut pause = WAIT_FIRST;
    let mut rest = line;
    while !rest.is_empty() {
        match writer.write(rest) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => {
                rest = &rest[written..];
                pause = WAIT_FIRST;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                wait(pause);
                pause = (pause * 2).min(WAIT_MAX);
            }
            Err(e) => return Err(e),
        }
    }
    loop {
        match writer.flush() {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                wait(pause);
                pause = (pause * 2).min(WAIT_MAX);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::collections::VecDeque;
    use alloc::vec::Vec;

    /// What a [`Scripted`] writer does on one call of `write`.
    enum Step {
        /// Takes at most this many bytes.
        Take(usize),
        /// Fails with this kind of error.
        Fail(io::ErrorKind),
    }

    /// A writer that does what its script says, call after call, and
    /// panics when it is called after the script has run out, so that a
    /// test sees a write tried again after an error that should have ended
    /// it, or a loop that does not end.
    struct Scripted {
        writes: VecDeque<Step>,
        flushes: VecDeque<Option<io::ErrorKind>>,
        written: Vec<u8>,
    }

    impl Scripted {
        fn new(writes: Vec<Step>, flushes: Vec<Option<io::ErrorKind>>) -> Self {
            Self {
                writes: writes.into(),
                flushes: flushes.into(),
                written: Vec::new(),
            }
        }
    }

    impl Write for Scripted {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            match self
                .writes
                .pop_front()
                .expect("write called after the script")
            {
                Step::Take(n) => {
                    let n = n.min(buf.len());
                    self.written.extend_from_slice(&buf[..n]);
                    Ok(n)
                }
                Step::Fail(kind) => Err(kind.into()),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            match self
                .flushes
                .pop_front()
                .expect("flush called after the script")
            {
                None => Ok(()),
                Some(kind) => Err(kind.into()),
            }
        }
    }

    /// Runs [`write_line_to`] on `writer` with `line`, and returns its
    /// result with the waits it asked for, in milliseconds.
    fn run(writer: &mut Scripted, line: &[u8]) -> (io::Result<()>, Vec<u128>) {
        let mut waits = Vec::new();
        let result = write_line_to(writer, line, &mut |d: Duration| waits.push(d.as_millis()));
        (result, waits)
    }

    #[test]
    fn waits_while_the_writer_would_block() {
        let mut writer = Scripted::new(
            vec![
                Step::Fail(io::ErrorKind::WouldBlock),
                Step::Fail(io::ErrorKind::WouldBlock),
                Step::Fail(io::ErrorKind::WouldBlock),
                Step::Take(usize::MAX),
            ],
            vec![None],
        );
        let (result, waits) = run(&mut writer, b"iter: 0\n");
        assert!(result.is_ok());
        assert_eq!(writer.written, b"iter: 0\n");
        assert_eq!(waits, [1, 2, 4]);
    }

    #[test]
    fn writes_the_rest_after_a_partial_write() {
        let mut writer = Scripted::new(
            vec![
                Step::Take(3),
                Step::Fail(io::ErrorKind::WouldBlock),
                Step::Take(usize::MAX),
            ],
            vec![None],
        );
        let (result, waits) = run(&mut writer, b"iter: 0\n");
        assert!(result.is_ok());
        assert_eq!(writer.written, b"iter: 0\n");
        assert_eq!(waits, [1]);
    }

    #[test]
    fn tries_an_interrupted_write_again_at_once() {
        let mut writer = Scripted::new(
            vec![
                Step::Fail(io::ErrorKind::Interrupted),
                Step::Take(usize::MAX),
            ],
            vec![None],
        );
        let (result, waits) = run(&mut writer, b"iter: 0\n");
        assert!(result.is_ok());
        assert_eq!(writer.written, b"iter: 0\n");
        assert!(waits.is_empty());
    }

    #[test]
    fn drops_the_line_on_other_errors() {
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::Other] {
            // The script has one step: a second try would panic.
            let mut writer = Scripted::new(vec![Step::Fail(kind)], vec![]);
            let (result, waits) = run(&mut writer, b"iter: 0\n");
            assert_eq!(result.map_err(|e| e.kind()), Err(kind), "{kind:?}");
            assert!(writer.written.is_empty(), "{kind:?}");
            assert!(waits.is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn gives_up_when_the_writer_takes_nothing() {
        let mut writer = Scripted::new(vec![Step::Take(0)], vec![]);
        let (result, _) = run(&mut writer, b"iter: 0\n");
        assert_eq!(result.map_err(|e| e.kind()), Err(io::ErrorKind::WriteZero));
    }

    #[test]
    fn flushes_again_while_the_flush_would_block() {
        let mut writer = Scripted::new(
            vec![Step::Take(usize::MAX)],
            vec![
                Some(io::ErrorKind::WouldBlock),
                Some(io::ErrorKind::Interrupted),
                None,
            ],
        );
        let (result, waits) = run(&mut writer, b"iter: 0\n");
        assert!(result.is_ok());
        assert_eq!(waits, [1]);
        assert!(writer.flushes.is_empty());
    }

    #[test]
    fn caps_the_wait_and_resets_it_after_progress() {
        let mut steps: Vec<Step> = (0..8)
            .map(|_| Step::Fail(io::ErrorKind::WouldBlock))
            .collect();
        steps.push(Step::Take(1));
        steps.push(Step::Fail(io::ErrorKind::WouldBlock));
        steps.push(Step::Take(usize::MAX));
        let mut writer = Scripted::new(steps, vec![None]);
        let (result, waits) = run(&mut writer, b"iter: 0\n");
        assert!(result.is_ok());
        assert_eq!(writer.written, b"iter: 0\n");
        assert_eq!(waits, [1, 2, 4, 8, 16, 32, 50, 50, 1]);
    }
}

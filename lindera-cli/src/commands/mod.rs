use std::fmt;
use std::io::{self, Write};

use lindera::LinderaResult;
use lindera::error::{LinderaError, LinderaErrorKind};

pub mod build;
pub mod download;
#[cfg(feature = "train")]
pub mod export;
pub mod list;
pub mod tokenize;
#[cfg(feature = "train")]
pub mod train;

/// Maps an error into a `LinderaError` with the `Io` kind.
///
/// Shared by the subcommands' I/O calls (`.map_err(io_err)`).
pub(crate) fn io_err<E>(err: E) -> LinderaError
where
    E: std::error::Error + Send + Sync + 'static,
{
    LinderaErrorKind::Io.with_error(anyhow::anyhow!(err))
}

/// The error of a write to the standard output after its reader has exited,
/// as `head` does once it has the lines it wants (#1316).
///
/// A command ends quietly, with exit code 0, on this error: `main` looks for
/// it with [`is_stdout_closed`].
#[derive(Debug)]
pub(crate) struct StdoutClosed;

impl fmt::Display for StdoutClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the standard output has no reader")
    }
}

impl std::error::Error for StdoutClosed {}

/// Maps the error of a write to the standard output into a `LinderaError`
/// with the `Io` kind.
///
/// A broken pipe, which a write gets once the reader of the standard output
/// has exited, becomes [`StdoutClosed`]; any other error is mapped as
/// [`io_err`] maps it. Use it for writes to the standard output only: a broken
/// pipe elsewhere, such as on the socket of a download, is an error.
///
/// # Arguments
///
/// * `err` - The error of the write.
///
/// # Returns
///
/// The error to return.
pub(crate) fn stdout_err(err: io::Error) -> LinderaError {
    if err.kind() == io::ErrorKind::BrokenPipe {
        LinderaErrorKind::Io.with_error(StdoutClosed)
    } else {
        io_err(err)
    }
}

/// Writes `text` to the standard output and flushes it, so that an error of
/// the write is returned rather than dropped at exit.
///
/// # Arguments
///
/// * `text` - The text to write.
///
/// # Returns
///
/// `Ok(())` once the text is written, or the error of the write, mapped with
/// [`stdout_err`].
pub(crate) fn write_stdout(text: &str) -> LinderaResult<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes()).map_err(stdout_err)?;
    stdout.flush().map_err(stdout_err)
}

/// Tells whether an error comes from a write to the standard output that had
/// no reader, as [`stdout_err`] marks it.
///
/// The mark is the source of the error, so an error that has had a context
/// added to it with `add_context` is not recognized.
///
/// # Arguments
///
/// * `err` - The error.
///
/// # Returns
///
/// `true` if the source of `err` is [`StdoutClosed`].
pub(crate) fn is_stdout_closed(err: &LinderaError) -> bool {
    std::error::Error::source(err).is_some_and(|source| source.is::<StdoutClosed>())
}

#[cfg(test)]
mod tests {
    use std::io;

    use lindera::error::LinderaErrorKind;

    use super::{io_err, is_stdout_closed, stdout_err};

    #[test]
    fn a_broken_pipe_on_the_standard_output_is_marked() {
        let err = stdout_err(io::ErrorKind::BrokenPipe.into());
        assert_eq!(err.kind, LinderaErrorKind::Io);
        assert!(is_stdout_closed(&err), "{err:?}");
    }

    /// A broken pipe that does not come from the standard output, such as
    /// one on the socket of `lindera download`, stays an error.
    #[test]
    fn a_broken_pipe_elsewhere_is_not_marked() {
        let err = io_err(io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(!is_stdout_closed(&err), "{err:?}");
    }

    #[test]
    fn another_error_on_the_standard_output_is_not_marked() {
        let err = stdout_err(io::ErrorKind::PermissionDenied.into());
        assert_eq!(err.kind, LinderaErrorKind::Io);
        assert!(!is_stdout_closed(&err), "{err:?}");
    }
}

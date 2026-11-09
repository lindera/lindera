// As in main.rs: messages on stderr go through `write_stderr` and
// `write_stderr_line`, not `eprint!` and `eprintln!`, which panic when stderr
// has no reader (#1338).
#![deny(clippy::print_stderr)]

use std::fmt;
use std::io::{self, Write};

use env_logger::{Builder, Env};

/// The version of this crate, shown by `lindera --version`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The log filter used when `RUST_LOG` is not set: warnings and errors.
const DEFAULT_LOG_FILTER: &str = "warn";

/// Returns the version of this crate.
///
/// # Returns
///
/// The version, such as `7.0.0`.
pub fn get_version() -> &'static str {
    VERSION
}

/// Returns a logger builder that reads its filter and write style from
/// `env`, with [`DEFAULT_LOG_FILTER`] when the filter variable is not set.
///
/// The logger writes to stderr, env_logger's default target, so that stdout
/// carries only the output of the command.
///
/// # Arguments
///
/// * `env` - The environment variables to read the filter and the write
///   style from.
///
/// # Returns
///
/// The builder.
fn logger_builder(env: Env) -> Builder {
    Builder::from_env(env.default_filter_or(DEFAULT_LOG_FILTER))
}

/// Installs the logger that writes the records of the `log` crate, such as
/// the warnings of the library crates, to stderr.
///
/// `RUST_LOG` sets the filter, for example `error` to hide the warnings or
/// `debug` for more detail; without it, warnings and errors are written.
/// `RUST_LOG_STYLE` sets whether the output is colored (`auto` by default).
///
/// # Panics
///
/// Panics if a logger has already been installed in this process, so call it
/// once, at the start of `main`.
pub fn init_logger() {
    logger_builder(Env::default()).init();
}

/// Writes a message to stderr, without a newline, and ignores a failed
/// write, so that the command goes on when stderr has no reader (#1338).
///
/// The CLI's own messages on stderr, such as the progress of
/// `lindera download`, go through this and [`write_stderr_line`] rather than
/// `eprint!` and `eprintln!`, which panic when the write fails.
/// `lindera_crf::console::write_line`, which the training's lines use, also
/// waits while a non-blocking stderr is full; this drops the message instead.
/// It is to be replaced with `write_line` once `lindera-crf` exposes it
/// without the training (#1318, #1329).
///
/// # Arguments
///
/// * `args` - The message, as `format_args!` makes it.
pub fn write_stderr(args: fmt::Arguments<'_>) {
    write_ignoring_errors(&mut io::stderr().lock(), args, false);
}

/// Writes a message and a newline to stderr, and ignores a failed write, as
/// [`write_stderr`] does.
///
/// # Arguments
///
/// * `args` - The message, without the newline, as `format_args!` makes it.
pub fn write_stderr_line(args: fmt::Arguments<'_>) {
    write_ignoring_errors(&mut io::stderr().lock(), args, true);
}

/// Writes `args`, and a newline after it when `newline` is set, to `writer`,
/// and flushes it. The first error ends the write and is ignored.
///
/// # Arguments
///
/// * `writer` - The writer.
/// * `args` - The message.
/// * `newline` - Whether to write a newline after the message.
fn write_ignoring_errors<W: Write + ?Sized>(
    writer: &mut W,
    args: fmt::Arguments<'_>,
    newline: bool,
) {
    let mut write = || -> io::Result<()> {
        writer.write_fmt(args)?;
        if newline {
            writer.write_all(b"\n")?;
        }
        writer.flush()
    };
    // A message that cannot be written is dropped, as described above.
    let _ = write();
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use env_logger::Env;

    use super::{logger_builder, write_ignoring_errors};

    #[test]
    fn logger_writes_warnings_without_a_filter_variable() {
        // No environment sets this variable, so the default filter applies.
        let env = Env::new().filter("LINDERA_CLI_TEST_UNSET_LOG_FILTER");
        assert_eq!(logger_builder(env).build().filter(), log::LevelFilter::Warn);
    }

    /// A writer whose reader has gone, as a pipe whose reader has exited.
    struct BrokenPipe;

    impl Write for BrokenPipe {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn a_message_that_cannot_be_written_is_dropped() {
        // Returns rather than panicking, with or without the newline.
        write_ignoring_errors(&mut BrokenPipe, format_args!("Downloading {}", "x"), true);
        write_ignoring_errors(&mut BrokenPipe, format_args!("\r{}%", 50), false);
    }

    #[test]
    fn a_line_ends_with_a_newline() {
        let mut line = Vec::new();
        write_ignoring_errors(&mut line, format_args!("Skipped {}", "x"), true);
        assert_eq!(line, b"Skipped x\n");

        let mut progress = Vec::new();
        write_ignoring_errors(&mut progress, format_args!("\r{}%", 50), false);
        assert_eq!(progress, b"\r50%");
    }
}

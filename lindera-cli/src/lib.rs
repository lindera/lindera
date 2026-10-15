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

#[cfg(test)]
mod tests {
    use env_logger::Env;

    use super::logger_builder;

    #[test]
    fn logger_writes_warnings_without_a_filter_variable() {
        // No environment sets this variable, so the default filter applies.
        let env = Env::new().filter("LINDERA_CLI_TEST_UNSET_LOG_FILTER");
        assert_eq!(logger_builder(env).build().filter(), log::LevelFilter::Warn);
    }
}

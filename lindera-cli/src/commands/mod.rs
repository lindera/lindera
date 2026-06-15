use lindera::error::{LinderaError, LinderaErrorKind};

pub mod build;
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

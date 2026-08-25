#[cfg(feature = "embed-sudachidict")]
pub mod embedded;

/// Dictionary name used as the host of `embedded://` dictionary URIs.
pub const DICTIONARY_NAME: &str = "sudachidict";
/// Crate version, exposed through [`get_version`].
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of this crate.
///
/// # Returns
///
/// The crate version string (e.g. `"5.3.0"`).
pub fn get_version() -> &'static str {
    VERSION
}

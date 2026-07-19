pub mod dictionary;
pub mod error;
pub mod mode;
pub mod segmenter;
pub mod token;

pub type LinderaResult<T> = lindera_dictionary::LinderaResult<T>;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}

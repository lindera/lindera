use std::path::Path;

use lindera_dictionary::LinderaResult;
use lindera_dictionary::dictionary::Dictionary;
use lindera_dictionary::error::LinderaErrorKind;

/// Load Ko-Dic dictionary from a directory path
pub fn load_from_path<P: AsRef<Path>>(dict_path: P) -> LinderaResult<Dictionary> {
    Dictionary::load_from_path(dict_path.as_ref())
}

/// Load Ko-Dic dictionary from default locations
pub fn load() -> LinderaResult<Dictionary> {
    // Search for dictionary in common locations
    let search_paths: [&'static str; 4] = [
        "./dict/ko-dic",
        "./lindera-ko-dic",
        "/usr/local/share/lindera/ko-dic",
        "/usr/share/lindera/ko-dic",
    ];

    for path in &search_paths {
        let dict_path = Path::new(path);
        if dict_path.exists() && dict_path.is_dir() {
            return load_from_path(dict_path);
        }
    }

    // If environment variable is set, use that
    if let Ok(dict_path) = std::env::var("LINDERA_KO_DIC_PATH") {
        let path = Path::new(&dict_path);
        if path.exists() {
            return load_from_path(path);
        }
    }

    Err(LinderaErrorKind::Io.with_error(anyhow::anyhow!(
        "Ko-Dic dictionary not found. Please set LINDERA_KO_DIC_PATH environment variable or place dictionary files in one of these locations: {}",
        search_paths.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore] // Requires actual dictionary files
    fn test_load_from_path() {
        // This test would require actual dictionary files
        // Skip in normal test runs
    }
}

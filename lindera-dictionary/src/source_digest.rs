//! Digest of this crate's source, which tells whether a cached dictionary was
//! built by the current dictionary builder (#1132).
//!
//! `build.rs` includes this file with `#[path]` and passes the digest to the
//! crate as `LINDERA_DICTIONARY_SOURCE_DIGEST`; the library includes it only
//! for its unit tests. A build script cannot use the crate it builds, so this
//! file depends on `std` and `md5` alone.

use std::fs;
use std::io;
use std::path::Path;

/// Reads every file named by `entries`, descending into directories.
///
/// Symbolic links are followed. Each file is named by its path relative to
/// `root`, with the components joined by `/` on every platform, so that the
/// digest does not depend on the path separator.
///
/// # Arguments
///
/// * `root` - The directory the entries are relative to (the crate root).
/// * `entries` - Files or directories under `root`, such as `src` and
///   `Cargo.toml`.
///
/// # Returns
///
/// The relative path and contents of every file found, in no particular
/// order, or the first I/O error, such as an entry that does not exist.
pub fn collect_files(root: &Path, entries: &[&str]) -> io::Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    let mut pending: Vec<(std::path::PathBuf, String)> = entries
        .iter()
        .map(|entry| (root.join(entry), (*entry).to_string()))
        .collect();
    while let Some((path, name)) = pending.pop() {
        if fs::metadata(&path)?.is_dir() {
            for child in fs::read_dir(&path)? {
                let child = child?;
                let child_name = format!("{name}/{}", child.file_name().to_string_lossy());
                pending.push((child.path(), child_name));
            }
        } else {
            files.push((name, fs::read(&path)?));
        }
    }
    Ok(files)
}

/// Computes the digest of a set of files.
///
/// The files are sorted by name first, so the order they were read in does
/// not matter. Each name and each content is preceded by its length, so that
/// moving bytes from one file to the next, or from a name into a content,
/// changes the digest.
///
/// # Arguments
///
/// * `files` - The relative path and contents of each file, as returned by
///   [`collect_files`].
///
/// # Returns
///
/// The MD5 digest as 32 lowercase hexadecimal digits.
pub fn digest_files(mut files: Vec<(String, Vec<u8>)>) -> String {
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut context = md5::Context::new();
    for (name, contents) in &files {
        context.consume((name.len() as u64).to_le_bytes());
        context.consume(name.as_bytes());
        context.consume((contents.len() as u64).to_le_bytes());
        context.consume(contents);
    }
    format!("{:x}", context.finalize())
}

#[cfg(test)]
mod tests {
    use super::{collect_files, digest_files};
    use std::fs;
    use std::path::Path;

    /// Writes `files` (relative path, contents) under `root`.
    fn write_tree(root: &Path, files: &[(&str, &str)]) {
        for (name, contents) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    /// The digest of `src` and `Cargo.toml` under `root`.
    fn tree_digest(root: &Path) -> String {
        digest_files(collect_files(root, &["src", "Cargo.toml"]).unwrap())
    }

    /// A small crate: two source files, one of them in a sub-directory.
    const TREE: [(&str, &str); 3] = [
        ("Cargo.toml", "[package]\n"),
        ("src/lib.rs", "pub mod builder;\n"),
        ("src/builder/prefix.rs", "fn build() {}\n"),
    ];

    #[test]
    fn same_tree_same_digest() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        write_tree(a.path(), &TREE);
        write_tree(b.path(), &TREE);
        // The digest names files relative to the root, so where the tree
        // lives does not matter.
        assert_eq!(tree_digest(a.path()), tree_digest(b.path()));
    }

    #[test]
    fn content_edit_of_the_same_length_changes_the_digest() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(dir.path(), &TREE);
        let before = tree_digest(dir.path());
        write_tree(dir.path(), &[("src/builder/prefix.rs", "fn build() {;\n")]);
        assert_ne!(tree_digest(dir.path()), before);
    }

    #[test]
    fn file_added_in_a_subdirectory_changes_the_digest() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(dir.path(), &TREE);
        let before = tree_digest(dir.path());
        write_tree(dir.path(), &[("src/builder/unknown.rs", "")]);
        assert_ne!(tree_digest(dir.path()), before);
    }

    #[test]
    fn rename_changes_the_digest() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(dir.path(), &TREE);
        let before = tree_digest(dir.path());
        fs::rename(
            dir.path().join("src/builder/prefix.rs"),
            dir.path().join("src/builder/prefiy.rs"),
        )
        .unwrap();
        assert_ne!(tree_digest(dir.path()), before);
    }

    #[test]
    fn digest_does_not_depend_on_the_input_order() {
        let files = vec![
            ("a".to_string(), b"1".to_vec()),
            ("b".to_string(), b"2".to_vec()),
        ];
        let reversed = files.iter().rev().cloned().collect();
        assert_eq!(digest_files(files), digest_files(reversed));
    }

    #[test]
    fn moving_bytes_between_name_and_contents_changes_the_digest() {
        // Without the length prefixes both would hash the bytes "abc".
        let ab_c = vec![("ab".to_string(), b"c".to_vec())];
        let a_bc = vec![("a".to_string(), b"bc".to_vec())];
        assert_ne!(digest_files(ab_c), digest_files(a_bc));
    }

    #[test]
    fn missing_entry_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(collect_files(dir.path(), &["src"]).is_err());
    }
}

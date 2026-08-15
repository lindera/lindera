//! The embedded (`include_bytes!`) dictionaries must read their connection
//! cost matrix in place, like the mmap path does.
//!
//! `include_bytes!` guarantees only 1-byte alignment, so `embedded_dictionary!`
//! routes `matrix.mtx` through `include_bytes_aligned!` to raise it. Without
//! that, `ConnectionCostMatrix::load` silently falls back to decoding the
//! payload into an owned `Vec<i16>` -- correct, but it reintroduces exactly
//! the copy #926 removes, and on a `no_std`-free target such as wasm32 (where
//! there is no mmap at all) the embedded path is the only path.

#![cfg(any(feature = "embed-ipadic", feature = "embed-unidic"))]

use lindera::dictionary::load_dictionary;

/// Asserts that the embedded dictionary behind `uri` borrows its matrix.
fn assert_embedded_matrix_is_borrowed(uri: &str) {
    let dictionary = load_dictionary(uri).expect("embedded dictionary should load");
    let matrix = &dictionary.connection_cost_matrix;

    assert!(
        matrix.is_zero_copy(),
        "{uri}: the embedded connection cost matrix must be borrowed, not copied. \
         The payload starts at byte 6 of matrix.mtx, so the backing static needs \
         at least 2-byte alignment -- check that embedded_dictionary! still uses \
         include_bytes_aligned! for CONNECTION_DATA."
    );

    // Sanity-check that the borrowed view really is the matrix: a MeCab
    // dictionary always has a defined cost between the BOS/EOS context ids.
    assert_eq!(
        matrix.costs().len() as u32,
        matrix.forward_size * matrix.backward_size
    );
}

#[test]
#[cfg(feature = "embed-ipadic")]
fn embedded_ipadic_matrix_is_zero_copy() {
    assert_embedded_matrix_is_borrowed("embedded://ipadic");
}

#[test]
#[cfg(feature = "embed-unidic")]
fn embedded_unidic_matrix_is_zero_copy() {
    assert_embedded_matrix_is_borrowed("embedded://unidic");
}

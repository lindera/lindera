//! Exports for the tests of the binding, compiled only with the `test-hooks`
//! feature, which `make test-lindera-nodejs` enables and a release build
//! never does.
//!
//! The functions here are left out of `index.d.ts` (`skip_typescript`), and
//! so out of the `module.exports` assignments of `index.js` too, which keeps
//! the committed entry points the same with and without the feature. They are
//! still properties of the loaded module, because the loader exports the
//! native binding itself.

/// Panics with `message`, so that tests/test_panic.js can check that a panic
/// in Rust throws an `Error` instead of aborting Node.js (#1314).
///
/// # Arguments
///
/// * `message` - The message of the panic.
#[napi(catch_unwind, skip_typescript, js_name = "__panicForTest")]
pub fn panic_for_test(message: String) {
    panic!("{message}");
}

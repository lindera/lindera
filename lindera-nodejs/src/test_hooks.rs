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

/// A task that panics in `compute`, through the function every batch task's
/// `compute` goes through (`compute_batch` in the `batch_task` module).
pub struct PanicTask {
    /// The message of the panic.
    message: String,
}

#[napi]
impl napi::Task for PanicTask {
    type Output = ();
    type JsValue = ();

    /// Panics with the message, on a thread of libuv's pool.
    fn compute(&mut self) -> napi::Result<Self::Output> {
        let message = std::mem::take(&mut self.message);
        crate::batch_task::compute_batch(|| -> napi::Result<()> { panic!("{message}") })
    }

    /// Never runs, as `compute` panics.
    fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(output)
    }
}

/// Returns a `Promise` whose task panics in `compute` with `message`, so
/// that tests/test_tokenize_batch_async.js can check that a panic in a task
/// rejects the `Promise` instead of aborting Node.js (#1406).
///
/// # Arguments
///
/// * `message` - The message of the panic.
///
/// # Returns
///
/// The task, as a `Promise`.
#[napi(catch_unwind, skip_typescript, js_name = "__panicInTaskForTest")]
pub fn panic_in_task_for_test(message: String) -> napi::bindgen_prelude::AsyncTask<PanicTask> {
    napi::bindgen_prelude::AsyncTask::new(PanicTask { message })
}

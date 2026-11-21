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
/// `compute` goes through (`catch_panic` in the `batch_task` module).
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
        crate::batch_task::catch_panic(|| -> napi::Result<()> { panic!("{message}") })
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

/// Sets the budget of a part for the batch tasks made from now on, so that
/// tests/test_tokenize_batch_async.js can make a small batch in many parts
/// (#1408).
///
/// # Arguments
///
/// * `units` - The weight that one part makes at most, 1 or more, or `null`
///   for the default (`PART_BUDGET` in the `batch_parts` module).
///
/// # Returns
///
/// `Ok(())`, or an `InvalidArg` error for 0.
#[napi(catch_unwind, skip_typescript, js_name = "__setBatchPartBudgetForTest")]
pub fn set_batch_part_budget_for_test(units: Option<u32>) -> napi::Result<()> {
    if units == Some(0) {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            "the budget of a part must be 1 or more",
        ));
    }
    crate::batch_parts::hooks::BUDGET.store(
        units.unwrap_or(0) as usize,
        std::sync::atomic::Ordering::Relaxed,
    );
    Ok(())
}

/// Returns the number of parts that batch tasks began in this process, after
/// their `resolve` (#1408).
///
/// # Returns
///
/// The number of parts.
#[napi(catch_unwind, skip_typescript, js_name = "__batchPartsMadeForTest")]
pub fn batch_parts_made_for_test() -> u32 {
    crate::batch_parts::hooks::PARTS_MADE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Makes a part of the next batch task fail, once (#1408).
///
/// # Arguments
///
/// * `part` - The part: 0 is the task's `resolve`, 1 the first part made
///   after it.
/// * `kind` - `"error"` (the part returns an error), `"panic"` (it panics)
///   or `"throw"` (it throws a JavaScript exception, then returns an error).
///
/// # Returns
///
/// `Ok(())`, or an `InvalidArg` error for another `kind`.
#[napi(catch_unwind, skip_typescript, js_name = "__failBatchPartForTest")]
pub fn fail_batch_part_for_test(part: u32, kind: String) -> napi::Result<()> {
    use crate::batch_parts::{Fault, FaultKind};
    let kind = match kind.as_str() {
        "error" => FaultKind::Error,
        "panic" => FaultKind::Panic,
        "throw" => FaultKind::Throw,
        other => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!("unknown kind of failure {other:?}"),
            ));
        }
    };
    let mut fault = crate::batch_parts::hooks::FAULT
        .lock()
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    *fault = Some(Fault { part, kind });
    Ok(())
}

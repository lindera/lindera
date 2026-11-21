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

/// Tokenizes `texts` as `tokenizeBatch` does, and makes the token objects
/// with the conversion that napi-rs derives from `Token`, as the binding
/// did before #1409, so that tests/test_token_objects.js can compare the
/// objects of `crate::token_objects` with them.
///
/// # Arguments
///
/// * `tokenizer` - The tokenizer.
/// * `texts` - The texts.
///
/// # Returns
///
/// The token objects of each text.
#[napi(
    catch_unwind,
    skip_typescript,
    js_name = "__tokenizeBatchDerivedForTest"
)]
pub fn tokenize_batch_derived_for_test(
    tokenizer: &crate::tokenizer::JsTokenizer,
    texts: Vec<String>,
) -> napi::Result<Vec<Vec<crate::token::Token>>> {
    let batch = tokenizer
        .inner
        .tokenize_batch(&texts, None)
        .map_err(crate::error::to_napi_error)?;
    Ok(batch
        .into_iter()
        .map(|views| {
            views
                .into_iter()
                .map(crate::token::Token::from_view)
                .collect()
        })
        .collect())
}

/// Tokenizes `text` as `tokenizeNbest(text, n)` does, and makes the N-best
/// results with the conversion that napi-rs derives from `NbestResult`, as
/// [`tokenize_batch_derived_for_test`] does for tokens.
///
/// # Arguments
///
/// * `tokenizer` - The tokenizer.
/// * `text` - The text.
/// * `n` - The number of results.
///
/// # Returns
///
/// The N-best results.
#[napi(
    catch_unwind,
    skip_typescript,
    js_name = "__tokenizeNbestDerivedForTest"
)]
pub fn tokenize_nbest_derived_for_test(
    tokenizer: &crate::tokenizer::JsTokenizer,
    text: String,
    n: u32,
) -> napi::Result<Vec<crate::token::NbestResult>> {
    let results = tokenizer
        .inner
        .tokenize_nbest(&text, n as usize, false, None)
        .map_err(crate::error::to_napi_error)?;
    Ok(results
        .into_iter()
        .map(|(views, cost)| crate::token::NbestResult {
            tokens: views
                .into_iter()
                .map(crate::token::Token::from_view)
                .collect(),
            cost,
        })
        .collect())
}

/// Sets the number of tokens made in one handle scope (#1409), so that the
/// tests can make small calls in many scopes.
///
/// # Arguments
///
/// * `tokens` - The tokens of a scope, 1 or more, or `null` for the default
///   (`SCOPE_TOKENS` in the `token_objects` module).
///
/// # Returns
///
/// `Ok(())`, or an `InvalidArg` error for 0.
#[napi(catch_unwind, skip_typescript, js_name = "__setTokenScopeForTest")]
pub fn set_token_scope_for_test(tokens: Option<u32>) -> napi::Result<()> {
    if tokens == Some(0) {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            "the tokens of a scope must be 1 or more",
        ));
    }
    crate::token_objects::hooks::SCOPE.store(
        tokens.unwrap_or(0) as usize,
        std::sync::atomic::Ordering::Relaxed,
    );
    Ok(())
}

/// Sets where the views of the calls are dropped once their objects are
/// made (#1409).
///
/// # Arguments
///
/// * `mode` - `"thread"` (always on the thread that drops them), `"inline"`
///   (always on the spot) or `null` (as the number of tokens says).
///
/// # Returns
///
/// `Ok(())`, or an `InvalidArg` error for another `mode`.
#[napi(catch_unwind, skip_typescript, js_name = "__setDropThreadForTest")]
pub fn set_drop_thread_for_test(mode: Option<String>) -> napi::Result<()> {
    let value = match mode.as_deref() {
        None => 0,
        Some("thread") => 1,
        Some("inline") => 2,
        Some(other) => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                format!("unknown place to drop the views {other:?}"),
            ));
        }
    };
    crate::token_objects::hooks::DROP.store(value, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

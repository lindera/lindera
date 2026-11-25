//! Making the results of the batch methods that return a `Promise` in parts,
//! with a turn of the event loop between two parts (#1408).
//!
//! A batch task's `resolve` (`crate::batch_task`) runs on the JavaScript
//! thread. When the weight of the results (see `Weight`) fits in the
//! budget of a part, it makes them all at once, as the synchronous methods
//! do. Otherwise it makes none and returns a thenable, `{ then: start }`,
//! which the task's promise adopts: V8 calls `start(resolve, reject)` in a
//! promise job, and `start` makes the outer array and calls
//! `setImmediate(step, step, outer, resolve, reject)`. Each call of `step`
//! makes one part, then calls `setImmediate` the same way, or
//! `resolve(outer)`. An immediate added while immediates run waits for the
//! next iteration of the event loop, so timers and I/O run between two parts.
//!
//! The only JavaScript values kept from one part to the next are the
//! arguments of the pending immediate; the views left are in the closure of
//! `step`. There is no `napi_deferred`, which only settling frees (a worker
//! terminated with parts pending would leak it), and no `napi_ref`.
//!
//! A part of token objects or N-best results is made by the conversion of
//! the synchronous methods (`crate::token_objects`), straight into the
//! outer array or the open text's array. A part of surfaces goes through
//! `surfaces_from_views` and napi's conversion of a `Vec` into a temporary
//! array, whose elements then move into the outer array.
//!
//! An error or a panic in `start` or `step` rejects the promise: an error
//! that a callback of `setImmediate` returned to napi would be thrown as an
//! uncaught exception, which ends Node.js.

use std::cell::RefCell;
use std::ffi::CStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::rc::Rc;

use napi::bindgen_prelude::{
    FromNapiValue, FunctionCallContext, ToNapiValue, TypeName, panic_to_error,
};
use napi::{Env, ValueType, check_status, sys};

use lindera_binding::TokenView;
use lindera_binding::token::SurfaceView;
use lindera_binding::tokenizer::NbestViews;

use crate::js::{element, length, set_element};
use crate::token::surfaces_from_views;
use crate::token_objects;

/// The weight of the values that one part makes at most, unless a single
/// value weighs more. About 1,700 IPADIC tokens or 32,768 surfaces.
pub(crate) const PART_BUDGET: usize = 32_768;

/// The weight of a text's array.
const TEXT_WEIGHT: usize = 1;

/// The weight of an N-best result's object, with its array of tokens.
const NBEST_RESULT_WEIGHT: usize = 2;

/// The weight of a token object, apart from its details.
const TOKEN_WEIGHT: usize = 10;

/// What a JavaScript value made from a view weighs, in units of about the
/// time of a surface string.
///
/// For the same 1.39 million IPADIC tokens, the token objects took about 19
/// times as long as the surface strings (#1408), so a token object weighs
/// [`TOKEN_WEIGHT`] plus one per detail.
pub(crate) trait Weight {
    /// The weight of the value made from `self`.
    fn weight(&self) -> usize;
}

impl Weight for TokenView {
    fn weight(&self) -> usize {
        TOKEN_WEIGHT + self.details.len()
    }
}

impl Weight for SurfaceView {
    fn weight(&self) -> usize {
        1
    }
}

/// A JavaScript value: what a batch task's `resolve` returns (the array of
/// the results, or the thenable that makes them in parts), and an argument
/// of the callbacks here.
pub struct BatchValue(sys::napi_value);

impl BatchValue {
    /// Wraps a JavaScript value.
    ///
    /// # Arguments
    ///
    /// * `value` - A handle of the current call.
    ///
    /// # Returns
    ///
    /// The wrapped value.
    pub(crate) fn new(value: sys::napi_value) -> Self {
        Self(value)
    }
}

impl ToNapiValue for BatchValue {
    unsafe fn to_napi_value(_env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        Ok(val.0)
    }
}

impl FromNapiValue for BatchValue {
    unsafe fn from_napi_value(_env: sys::napi_env, val: sys::napi_value) -> napi::Result<Self> {
        Ok(Self(val))
    }
}

impl TypeName for BatchValue {
    fn type_name() -> &'static str {
        "unknown"
    }

    fn value_type() -> ValueType {
        ValueType::Unknown
    }
}

/// How a batch is made in parts: the budget of a part and, in a build with
/// the `test-hooks` feature, a failure to inject. Read when a task is made.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PartConfig {
    /// The weight that one part makes at most.
    budget: usize,
    /// The failure to inject, if any.
    #[cfg(feature = "test-hooks")]
    fault: Option<Fault>,
}

impl PartConfig {
    /// The configuration of a task made now: [`PART_BUDGET`], or what the
    /// test hooks set.
    ///
    /// # Returns
    ///
    /// The configuration.
    pub(crate) fn current() -> Self {
        #[cfg(feature = "test-hooks")]
        {
            let budget = hooks::BUDGET.load(std::sync::atomic::Ordering::Relaxed);
            let fault = hooks::FAULT
                .lock()
                .map(|mut fault| fault.take())
                .unwrap_or(None);
            Self {
                budget: if budget == 0 { PART_BUDGET } else { budget },
                fault,
            }
        }
        #[cfg(not(feature = "test-hooks"))]
        Self {
            budget: PART_BUDGET,
        }
    }
}

/// A failure to inject into a part, for the tests (`crate::test_hooks`).
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fault {
    /// The part: 0 is the task's `resolve`, 1 the first call of `step`.
    pub(crate) part: u32,
    /// How the part fails.
    pub(crate) kind: FaultKind,
}

/// How an injected failure fails.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaultKind {
    /// The part returns an error.
    Error,
    /// The part panics.
    Panic,
    /// The part throws a JavaScript exception, then returns an error.
    Throw,
}

/// The state that the test hooks set and read.
#[cfg(feature = "test-hooks")]
pub(crate) mod hooks {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, AtomicUsize};

    use super::Fault;

    /// The budget of a part, or 0 for [`super::PART_BUDGET`].
    pub(crate) static BUDGET: AtomicUsize = AtomicUsize::new(0);

    /// The failure that the next task takes.
    pub(crate) static FAULT: Mutex<Option<Fault>> = Mutex::new(None);

    /// The number of parts that `step` began, in the whole process.
    pub(crate) static PARTS_MADE: AtomicU32 = AtomicU32::new(0);
}

/// Fails as the injected failure of `part` says, if there is one.
///
/// # Arguments
///
/// * `env` - The environment of the current call.
/// * `config` - The configuration of the batch.
/// * `part` - The part about to be made.
///
/// # Returns
///
/// `Ok(())`, or the error of the injected failure.
#[cfg(feature = "test-hooks")]
fn inject(env: sys::napi_env, config: &PartConfig, part: u32) -> napi::Result<()> {
    let Some(fault) = config.fault.filter(|fault| fault.part == part) else {
        return Ok(());
    };
    let message = format!("lindera-nodejs test failure in part {part}");
    match fault.kind {
        FaultKind::Error => Err(napi::Error::from_reason(message)),
        FaultKind::Panic => panic!("{message}"),
        FaultKind::Throw => {
            let text = std::ffi::CString::new(message)
                .map_err(|error| napi::Error::from_reason(error.to_string()))?;
            // SAFETY: `env` is the environment of the current call, and
            // `text` outlives the call.
            check_status!(unsafe { sys::napi_throw_error(env, ptr::null(), text.as_ptr()) })?;
            Err(napi::Error::from_reason("the part threw"))
        }
    }
}

#[cfg(not(feature = "test-hooks"))]
#[inline(always)]
fn inject(_env: sys::napi_env, _config: &PartConfig, _part: u32) -> napi::Result<()> {
    Ok(())
}

/// Counts a part that `step` begins, for `__batchPartsMadeForTest`.
#[inline(always)]
fn count_part() {
    #[cfg(feature = "test-hooks")]
    hooks::PARTS_MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the weight of a batch of texts fits in `budget`.
///
/// # Arguments
///
/// * `batch` - The views of each text.
/// * `budget` - The budget of a part.
///
/// # Returns
///
/// `true` when the arrays of the texts and the values of their views weigh
/// no more than `budget`.
fn flat_fits<V: Weight>(batch: &[Vec<V>], budget: usize) -> bool {
    let mut used = 0usize;
    for text in batch {
        used += TEXT_WEIGHT;
        for view in text {
            used += view.weight();
        }
        if used > budget {
            return false;
        }
    }
    true
}

/// Whether the weight of a batch of N-best results fits in `budget`, as
/// [`flat_fits`] says for texts.
///
/// # Arguments
///
/// * `batch` - The N-best results of each text.
/// * `budget` - The budget of a part.
///
/// # Returns
///
/// `true` when the texts' arrays, the results' objects and the token
/// objects weigh no more than `budget`.
fn nbest_fits(batch: &[NbestViews], budget: usize) -> bool {
    let mut used = 0usize;
    for text in batch {
        used += TEXT_WEIGHT;
        for (views, _) in text {
            used += NBEST_RESULT_WEIGHT;
            for view in views {
                used += view.weight();
            }
        }
        if used > budget {
            return false;
        }
    }
    true
}

/// Takes values from `values` while the part has room, or while it has not
/// made a value yet.
///
/// A part makes progress when it makes a value, or an empty text or result;
/// until then it takes the next value whatever its weight, so that a budget
/// of 1 still goes on.
///
/// # Arguments
///
/// * `values` - The values left.
/// * `budget` - The budget of the part.
/// * `used` - The weight the part has made.
/// * `progressed` - Whether the part has made progress.
///
/// # Returns
///
/// The values taken, in order.
fn take_values<V: Weight>(
    values: &mut std::vec::IntoIter<V>,
    budget: usize,
    used: &mut usize,
    progressed: &mut bool,
) -> Vec<V> {
    let mut taken = Vec::new();
    while *used < budget || !*progressed {
        let Some(value) = values.next() else {
            break;
        };
        *used += value.weight();
        *progressed = true;
        taken.push(value);
    }
    taken
}

/// The views of a part of a batch of texts.
pub(crate) struct FlatPart<V> {
    /// The number of texts started before this part: the first chunk that
    /// starts a text is the text at this index.
    pub(crate) first: usize,
    /// Whether the first chunk continues the text before `first`.
    pub(crate) continues: bool,
    /// The chunks, in order: each one starts a text, but the first one when
    /// `continues`.
    pub(crate) chunks: Vec<Vec<V>>,
}

/// The views of a batch of texts (tokens or surfaces) not made yet.
pub(crate) struct FlatCursor<V> {
    /// The texts not started.
    texts: std::vec::IntoIter<Vec<V>>,
    /// The views left of the text started last, if it is not done.
    open: Option<std::vec::IntoIter<V>>,
    /// The number of texts started.
    started: usize,
    /// The number of texts.
    len: usize,
}

impl<V: Weight> FlatCursor<V> {
    /// Makes the cursor of a batch.
    ///
    /// # Arguments
    ///
    /// * `batch` - The views of each text.
    ///
    /// # Returns
    ///
    /// The cursor, before the first part.
    pub(crate) fn new(batch: Vec<Vec<V>>) -> Self {
        let len = batch.len();
        Self {
            texts: batch.into_iter(),
            open: None,
            started: 0,
            len,
        }
    }

    /// Takes the views of the next part: those of the text left open first,
    /// then those of the next texts, up to `budget`. A text that does not
    /// fit is split, and the part ends there.
    ///
    /// # Arguments
    ///
    /// * `budget` - The budget of the part.
    ///
    /// # Returns
    ///
    /// The part, or `None` when every view has been taken.
    pub(crate) fn take(&mut self, budget: usize) -> Option<FlatPart<V>> {
        let first = self.started;
        let mut used = 0usize;
        let mut progressed = false;
        let mut chunks = Vec::new();
        let mut continues = false;
        if let Some(mut open) = self.open.take() {
            continues = true;
            chunks.push(take_values(&mut open, budget, &mut used, &mut progressed));
            if open.len() > 0 {
                self.open = Some(open);
                return Some(FlatPart {
                    first,
                    continues,
                    chunks,
                });
            }
        }
        while used < budget || !progressed {
            let Some(text) = self.texts.next() else {
                break;
            };
            self.started += 1;
            used += TEXT_WEIGHT;
            let weight: usize = text.iter().map(Weight::weight).sum();
            if text.is_empty() || used + weight <= budget {
                used += weight;
                progressed = true;
                chunks.push(text);
                continue;
            }
            let mut values = text.into_iter();
            chunks.push(take_values(&mut values, budget, &mut used, &mut progressed));
            if values.len() > 0 {
                self.open = Some(values);
                break;
            }
        }
        if chunks.is_empty() {
            None
        } else {
            Some(FlatPart {
                first,
                continues,
                chunks,
            })
        }
    }
}

/// The views of a part of a batch of N-best results.
pub(crate) struct NbestPart {
    /// The number of texts started before this part, as in [`FlatPart`].
    pub(crate) first: usize,
    /// Whether the first chunk continues the text before `first`.
    pub(crate) continues_text: bool,
    /// Whether the first result of the first chunk continues the last result
    /// of that text: its tokens go after those of that result, and its cost
    /// is not used.
    pub(crate) continues_result: bool,
    /// The chunks, in order, as in [`FlatPart`].
    pub(crate) chunks: Vec<NbestViews>,
}

/// The N-best results of a batch not made yet.
pub(crate) struct NbestCursor {
    /// The texts not started.
    texts: std::vec::IntoIter<NbestViews>,
    /// The results left of the text started last, if it is not done.
    open_text: Option<std::vec::IntoIter<(Vec<TokenView>, i64)>>,
    /// The views left of the result started last, if it is not done.
    open_result: Option<std::vec::IntoIter<TokenView>>,
    /// The number of texts started.
    started: usize,
    /// The number of texts.
    len: usize,
}

impl NbestCursor {
    /// Makes the cursor of a batch.
    ///
    /// # Arguments
    ///
    /// * `batch` - The N-best results of each text.
    ///
    /// # Returns
    ///
    /// The cursor, before the first part.
    pub(crate) fn new(batch: Vec<NbestViews>) -> Self {
        let len = batch.len();
        Self {
            texts: batch.into_iter(),
            open_text: None,
            open_result: None,
            started: 0,
            len,
        }
    }

    /// Takes the results of the next part, as [`FlatCursor::take`] does for
    /// texts; a result that does not fit is split too.
    ///
    /// # Arguments
    ///
    /// * `budget` - The budget of the part.
    ///
    /// # Returns
    ///
    /// The part, or `None` when every view has been taken.
    pub(crate) fn take(&mut self, budget: usize) -> Option<NbestPart> {
        let first = self.started;
        let mut used = 0usize;
        let mut progressed = false;
        let mut chunks = Vec::new();
        let mut continues_text = false;
        let mut continues_result = false;
        if let Some(mut results) = self.open_text.take() {
            continues_text = true;
            let mut chunk = Vec::new();
            if let Some(mut views) = self.open_result.take() {
                continues_result = true;
                let taken = take_values(&mut views, budget, &mut used, &mut progressed);
                chunk.push((taken, 0));
                if views.len() > 0 {
                    self.open_result = Some(views);
                    self.open_text = Some(results);
                    chunks.push(chunk);
                    return Some(NbestPart {
                        first,
                        continues_text,
                        continues_result,
                        chunks,
                    });
                }
            }
            let open =
                self.take_results(&mut results, &mut chunk, budget, &mut used, &mut progressed);
            chunks.push(chunk);
            if open || results.len() > 0 {
                self.open_text = Some(results);
                return Some(NbestPart {
                    first,
                    continues_text,
                    continues_result,
                    chunks,
                });
            }
        }
        while used < budget || !progressed {
            let Some(text) = self.texts.next() else {
                break;
            };
            self.started += 1;
            used += TEXT_WEIGHT;
            if text.is_empty() {
                progressed = true;
                chunks.push(text);
                continue;
            }
            let mut results = text.into_iter();
            let mut chunk = Vec::new();
            let open =
                self.take_results(&mut results, &mut chunk, budget, &mut used, &mut progressed);
            chunks.push(chunk);
            if open || results.len() > 0 {
                self.open_text = Some(results);
                break;
            }
        }
        if chunks.is_empty() {
            None
        } else {
            Some(NbestPart {
                first,
                continues_text,
                continues_result,
                chunks,
            })
        }
    }

    /// Takes results of a text into `chunk` while the part has room; a
    /// result that does not fit is split, and its views left stay open.
    ///
    /// # Arguments
    ///
    /// * `results` - The results left of the text.
    /// * `chunk` - The chunk of the text in this part.
    /// * `budget` - The budget of the part.
    /// * `used` - The weight the part has made.
    /// * `progressed` - Whether the part has made progress.
    ///
    /// # Returns
    ///
    /// Whether a result was split (the part then ends).
    fn take_results(
        &mut self,
        results: &mut std::vec::IntoIter<(Vec<TokenView>, i64)>,
        chunk: &mut NbestViews,
        budget: usize,
        used: &mut usize,
        progressed: &mut bool,
    ) -> bool {
        while *used < budget || !*progressed {
            let Some((views, cost)) = results.next() else {
                return false;
            };
            *used += NBEST_RESULT_WEIGHT;
            let weight: usize = views.iter().map(Weight::weight).sum();
            if views.is_empty() || *used + weight <= budget {
                *used += weight;
                *progressed = true;
                chunk.push((views, cost));
                continue;
            }
            let mut views = views.into_iter();
            chunk.push((take_values(&mut views, budget, used, progressed), cost));
            if views.len() > 0 {
                self.open_result = Some(views);
                return true;
            }
        }
        false
    }
}

/// Appends the elements of `source` from `from` on to `target`.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `target` and
/// `source` array handles of it.
unsafe fn append(
    env: sys::napi_env,
    target: sys::napi_value,
    source: sys::napi_value,
    from: u32,
) -> napi::Result<()> {
    // SAFETY: the caller guarantees the handles.
    let (start, len) = unsafe { (length(env, target)?, length(env, source)?) };
    for index in from..len {
        // SAFETY: as above; the element is a handle of the current call.
        unsafe {
            let value = element(env, source, index)?;
            set_element(env, target, start + index - from, value)?;
        }
    }
    Ok(())
}

/// Calls `function` with `args`, with `undefined` as `this`.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `function` and
/// every argument handles of it.
unsafe fn call(
    env: sys::napi_env,
    function: sys::napi_value,
    args: &[sys::napi_value],
) -> napi::Result<()> {
    let mut this = ptr::null_mut();
    let mut returned = ptr::null_mut();
    // SAFETY: the caller guarantees the handles; `this`, `args` and
    // `returned` outlive the calls.
    unsafe {
        check_status!(sys::napi_get_undefined(env, &mut this))?;
        check_status!(sys::napi_call_function(
            env,
            this,
            function,
            args.len(),
            args.as_ptr(),
            &mut returned,
        ))
    }
}

/// The global function `name`, or `None` when it is not a function.
///
/// # Safety
///
/// `env` must be the environment of the current call.
unsafe fn global_function(
    env: sys::napi_env,
    name: &CStr,
) -> napi::Result<Option<sys::napi_value>> {
    let mut global = ptr::null_mut();
    let mut function = ptr::null_mut();
    let mut kind = sys::ValueType::napi_undefined;
    // SAFETY: the caller guarantees `env`; the out-pointers and `name`
    // outlive the calls.
    unsafe {
        check_status!(sys::napi_get_global(env, &mut global))?;
        check_status!(sys::napi_get_named_property(
            env,
            global,
            name.as_ptr(),
            &mut function
        ))?;
        check_status!(sys::napi_typeof(env, function, &mut kind))?;
    }
    Ok((kind == sys::ValueType::napi_function).then_some(function))
}

/// Takes the pending JavaScript exception, if there is one.
///
/// # Safety
///
/// `env` must be the environment of the current call.
unsafe fn take_exception(env: sys::napi_env) -> Option<sys::napi_value> {
    let mut pending = false;
    let mut exception = ptr::null_mut();
    // SAFETY: the caller guarantees `env`; the out-pointers outlive the
    // calls.
    unsafe {
        if sys::napi_is_exception_pending(env, &mut pending) != sys::Status::napi_ok || !pending {
            return None;
        }
        if sys::napi_get_and_clear_last_exception(env, &mut exception) != sys::Status::napi_ok {
            return None;
        }
    }
    Some(exception)
}

/// The views of a batch not made yet, with the conversions that make a part
/// of them into the outer array.
pub(crate) trait Cursor: 'static {
    /// The number of texts of the batch: the length of the outer array.
    fn len(&self) -> usize;

    /// Makes the next part into `outer`.
    ///
    /// # Arguments
    ///
    /// * `env` - The environment of the current call.
    /// * `outer` - The outer array, of [`Cursor::len`] elements.
    /// * `budget` - The budget of the part.
    ///
    /// # Returns
    ///
    /// Whether every value is made.
    ///
    /// # Safety
    ///
    /// `env` must be the environment of the current call, and `outer` an
    /// array handle of it.
    unsafe fn make_part(
        &mut self,
        env: sys::napi_env,
        outer: sys::napi_value,
        budget: usize,
    ) -> napi::Result<bool>;
}

/// Puts a part, made into `part` (an array of one array per chunk), into
/// `outer`.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `outer` and
/// `part` array handles of it.
unsafe fn place_flat(
    env: sys::napi_env,
    outer: sys::napi_value,
    part: sys::napi_value,
    first: usize,
    continues: bool,
) -> napi::Result<()> {
    // SAFETY: the caller guarantees the handles; every element is a handle
    // of the current call.
    unsafe {
        let chunks = length(env, part)?;
        let mut next = first as u32;
        for index in 0..chunks {
            let chunk = element(env, part, index)?;
            if index == 0 && continues {
                append(env, element(env, outer, next - 1)?, chunk, 0)?;
            } else {
                set_element(env, outer, next, chunk)?;
                next += 1;
            }
        }
    }
    Ok(())
}

impl Cursor for FlatCursor<TokenView> {
    fn len(&self) -> usize {
        self.len
    }

    unsafe fn make_part(
        &mut self,
        env: sys::napi_env,
        outer: sys::napi_value,
        budget: usize,
    ) -> napi::Result<bool> {
        let Some(part) = self.take(budget) else {
            return Ok(true);
        };
        // SAFETY: the caller guarantees the handles.
        unsafe {
            token_objects::fill_texts(env, outer, part.first as u32, part.continues, &part.chunks)?;
        }
        let tokens = part.chunks.iter().map(Vec::len).sum();
        token_objects::drop_views(tokens, part.chunks);
        Ok(self.open.is_none() && self.texts.len() == 0)
    }
}

impl Cursor for FlatCursor<SurfaceView> {
    fn len(&self) -> usize {
        self.len
    }

    unsafe fn make_part(
        &mut self,
        env: sys::napi_env,
        outer: sys::napi_value,
        budget: usize,
    ) -> napi::Result<bool> {
        let Some(part) = self.take(budget) else {
            return Ok(true);
        };
        // SAFETY: the caller guarantees the handles; the part's array is a
        // handle of the current call.
        unsafe {
            let made = ToNapiValue::to_napi_value(env, surfaces_from_views(part.chunks))?;
            place_flat(env, outer, made, part.first, part.continues)?;
        }
        Ok(self.open.is_none() && self.texts.len() == 0)
    }
}

impl Cursor for NbestCursor {
    fn len(&self) -> usize {
        self.len
    }

    unsafe fn make_part(
        &mut self,
        env: sys::napi_env,
        outer: sys::napi_value,
        budget: usize,
    ) -> napi::Result<bool> {
        let Some(part) = self.take(budget) else {
            return Ok(true);
        };
        // SAFETY: the caller guarantees the handles.
        unsafe {
            token_objects::fill_nbest_texts(
                env,
                outer,
                part.first as u32,
                part.continues_text,
                part.continues_result,
                &part.chunks,
            )?;
        }
        let tokens = part
            .chunks
            .iter()
            .flat_map(|text| text.iter().map(|(views, _)| views.len()))
            .sum();
        token_objects::drop_views(tokens, part.chunks);
        Ok(self.open_text.is_none() && self.texts.len() == 0)
    }
}

/// The state of a batch made in parts, which `start` and `step` share.
struct Batch<C> {
    /// The views not made yet; `None` once the promise is settled.
    cursor: Option<C>,
    /// How the batch is made.
    config: PartConfig,
    /// The parts begun.
    parts: u32,
}

/// Makes every result of a batch that fits in one part, as the synchronous
/// method does: the value of `resolve`.
///
/// # Arguments
///
/// * `env` - The environment of `resolve`.
/// * `batch` - The views of the batch.
/// * `convert` - The conversion of the synchronous method.
///
/// # Returns
///
/// The array of the results.
fn at_once<B>(
    env: &Env,
    batch: B,
    convert: unsafe fn(sys::napi_env, B) -> napi::Result<sys::napi_value>,
) -> napi::Result<BatchValue> {
    // SAFETY: `env` is the environment of `resolve`, the current call.
    Ok(BatchValue(unsafe { convert(env.raw(), batch)? }))
}

/// Makes the value of `resolve` for a batch that does not fit in one part:
/// a thenable whose `then` makes the results in parts.
///
/// # Arguments
///
/// * `env` - The environment of `resolve`.
/// * `cursor` - The views of the batch.
/// * `config` - How the batch is made.
///
/// # Returns
///
/// The thenable.
fn in_parts<C: Cursor>(env: &Env, cursor: C, config: PartConfig) -> napi::Result<BatchValue> {
    let raw = env.raw();
    let batch = Rc::new(RefCell::new(Batch {
        cursor: Some(cursor),
        config,
        parts: 0,
    }));
    let start = env.create_function_from_closure::<(), (), _>("linderaBatchStart", move |ctx| {
        start(ctx, &batch);
        Ok(())
    })?;
    let mut thenable = ptr::null_mut();
    // SAFETY: `raw` is the environment of `resolve`, and the function and
    // the object are handles of this call.
    unsafe {
        let start = ToNapiValue::to_napi_value(raw, start)?;
        check_status!(sys::napi_create_object(raw, &mut thenable))?;
        check_status!(sys::napi_set_named_property(
            raw,
            thenable,
            c"then".as_ptr(),
            start
        ))?;
    }
    Ok(BatchValue(thenable))
}

/// Makes a batch of texts the value of `resolve`, with the conversion of
/// the synchronous method for a batch that fits in one part.
///
/// # Arguments
///
/// * `env` - The environment of `resolve`.
/// * `batch` - The views of each text.
/// * `convert` - The conversion of the synchronous method.
/// * `config` - How the batch is made.
///
/// # Returns
///
/// The value of `resolve`.
pub(crate) fn resolve_flat<V: Weight>(
    env: &Env,
    batch: Vec<Vec<V>>,
    convert: unsafe fn(sys::napi_env, Vec<Vec<V>>) -> napi::Result<sys::napi_value>,
    config: PartConfig,
) -> napi::Result<BatchValue>
where
    FlatCursor<V>: Cursor,
{
    inject(env.raw(), &config, 0)?;
    if flat_fits(&batch, config.budget) {
        at_once(env, batch, convert)
    } else {
        in_parts(env, FlatCursor::new(batch), config)
    }
}

/// Makes a batch of N-best results the value of `resolve`, as
/// [`resolve_flat`] does for texts.
///
/// # Arguments
///
/// * `env` - The environment of `resolve`.
/// * `batch` - The N-best results of each text.
/// * `config` - How the batch is made.
///
/// # Returns
///
/// The value of `resolve`.
pub(crate) fn resolve_nbest(
    env: &Env,
    batch: Vec<NbestViews>,
    config: PartConfig,
) -> napi::Result<BatchValue> {
    inject(env.raw(), &config, 0)?;
    if nbest_fits(&batch, config.budget) {
        at_once(env, batch, token_objects::nbest_batch)
    } else {
        in_parts(env, NbestCursor::new(batch), config)
    }
}

/// The `then` of the thenable: makes the outer array and schedules the
/// first part. Called by V8 with the promise's `resolve` and `reject`.
fn start<C: Cursor>(ctx: FunctionCallContext, batch: &Rc<RefCell<Batch<C>>>) {
    let env = ctx.env.raw();
    let (Ok(resolve), Ok(reject)) = (ctx.get::<BatchValue>(0), ctx.get::<BatchValue>(1)) else {
        return;
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| -> napi::Result<()> {
        let len = match batch.try_borrow() {
            Ok(state) => match &state.cursor {
                Some(cursor) => cursor.len(),
                None => return Ok(()),
            },
            Err(_) => return Ok(()),
        };
        let mut outer = ptr::null_mut();
        // SAFETY: `env` is the environment of this call; `outer` outlives
        // it.
        check_status!(unsafe { sys::napi_create_array_with_length(env, len, &mut outer) })?;
        let shared = Rc::clone(batch);
        let step =
            ctx.env
                .create_function_from_closure::<(), (), _>("linderaBatchPart", move |ctx| {
                    step(ctx, &shared);
                    Ok(())
                })?;
        // SAFETY: as above; the function is a handle of this call.
        let step = unsafe { ToNapiValue::to_napi_value(env, step)? };
        // SAFETY: every handle belongs to this call.
        unsafe { schedule(env, batch, step, outer, resolve.0, reject.0) }
    }));
    settle_failure(env, batch, reject.0, outcome);
}

/// A part of the batch: called by `setImmediate` with the function itself,
/// the outer array, and the promise's `resolve` and `reject`.
fn step<C: Cursor>(ctx: FunctionCallContext, batch: &Rc<RefCell<Batch<C>>>) {
    let env = ctx.env.raw();
    let (Ok(step), Ok(outer), Ok(resolve), Ok(reject)) = (
        ctx.get::<BatchValue>(0),
        ctx.get::<BatchValue>(1),
        ctx.get::<BatchValue>(2),
        ctx.get::<BatchValue>(3),
    ) else {
        return;
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| -> napi::Result<()> {
        let done = {
            // A call while a part runs, or after the promise is settled
            // (from code that reaches the immediate's callback), does
            // nothing.
            let Ok(mut state) = batch.try_borrow_mut() else {
                return Ok(());
            };
            let state = &mut *state;
            let Some(cursor) = state.cursor.as_mut() else {
                return Ok(());
            };
            state.parts += 1;
            count_part();
            inject(env, &state.config, state.parts)?;
            // SAFETY: `env` is the environment of this call, and `outer`
            // the array that `start` made, an argument of this call.
            let done = unsafe { cursor.make_part(env, outer.0, state.config.budget)? };
            if done {
                state.cursor = None;
            }
            done
        };
        // SAFETY: every handle belongs to this call.
        unsafe {
            if done {
                call(env, resolve.0, &[outer.0])
            } else {
                schedule(env, batch, step.0, outer.0, resolve.0, reject.0)
            }
        }
    }));
    settle_failure(env, batch, reject.0, outcome);
}

/// Schedules the next part with `setImmediate`, or, when the global
/// `setImmediate` is not a function, makes every part left at once and
/// resolves the promise.
///
/// # Safety
///
/// `env` must be the environment of the current call, and the other values
/// handles of it.
unsafe fn schedule<C: Cursor>(
    env: sys::napi_env,
    batch: &Rc<RefCell<Batch<C>>>,
    step: sys::napi_value,
    outer: sys::napi_value,
    resolve: sys::napi_value,
    reject: sys::napi_value,
) -> napi::Result<()> {
    // SAFETY: the caller guarantees the handles.
    unsafe {
        if let Some(set_immediate) = global_function(env, c"setImmediate")? {
            return call(env, set_immediate, &[step, step, outer, resolve, reject]);
        }
        {
            let Ok(mut state) = batch.try_borrow_mut() else {
                return Ok(());
            };
            if let Some(mut cursor) = state.cursor.take() {
                while !cursor.make_part(env, outer, usize::MAX)? {}
            }
        }
        call(env, resolve, &[outer])
    }
}

/// Rejects the promise when `start` or `step` failed or panicked, and ends
/// the batch. Never fails: an error returned to napi from a callback of
/// `setImmediate` would end Node.js.
///
/// # Arguments
///
/// * `env` - The environment of the current call.
/// * `batch` - The batch.
/// * `reject` - The promise's `reject`.
/// * `outcome` - What the callback's work returned, or its panic.
fn settle_failure<C: Cursor>(
    env: sys::napi_env,
    batch: &Rc<RefCell<Batch<C>>>,
    reject: sys::napi_value,
    outcome: std::thread::Result<napi::Result<()>>,
) {
    let error = match outcome {
        Ok(Ok(())) => return,
        Ok(Err(error)) => error,
        Err(panic) => panic_to_error(panic),
    };
    if let Ok(mut state) = batch.try_borrow_mut() {
        state.cursor = None;
    }
    // SAFETY: `env` is the environment of the current call, and `reject` a
    // handle of it; the value made of the error is one too.
    unsafe {
        let reason =
            take_exception(env).unwrap_or_else(|| napi::JsError::from(error).into_value(env));
        if call(env, reject, &[reason]).is_err() {
            take_exception(env);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(surface: &str, details: usize) -> TokenView {
        TokenView {
            surface: surface.to_string(),
            byte_start: 0,
            byte_end: surface.len(),
            position: 0,
            word_id: 0,
            is_unknown: false,
            details: vec!["*".to_string(); details],
            dependency: None,
        }
    }

    /// Texts of `sizes` tokens each, with surfaces `"<text>.<token>"`.
    fn texts(sizes: &[usize]) -> Vec<Vec<TokenView>> {
        sizes
            .iter()
            .enumerate()
            .map(|(t, &n)| (0..n).map(|i| token(&format!("{t}.{i}"), 2)).collect())
            .collect()
    }

    fn surfaces(text: &[TokenView]) -> Vec<String> {
        text.iter().map(|view| view.surface.clone()).collect()
    }

    /// Rebuilds the texts from the parts, as `place_flat` does, and checks
    /// that a part goes over the budget by less than the weight of one token
    /// and one text's array: a part takes a value only while it is under the
    /// budget, or while it has made nothing.
    fn rebuild(batch: Vec<Vec<TokenView>>, budget: usize) -> (Vec<Vec<String>>, usize) {
        let len = batch.len();
        let mut cursor = FlatCursor::new(batch);
        let mut rebuilt: Vec<Vec<String>> = Vec::new();
        let mut parts = 0;
        while let Some(part) = cursor.take(budget) {
            parts += 1;
            assert_eq!(
                part.first,
                rebuilt.len(),
                "part {parts} starts at the wrong text"
            );
            let mut weight = 0;
            for (index, chunk) in part.chunks.iter().enumerate() {
                if index == 0 && part.continues {
                    rebuilt
                        .last_mut()
                        .expect("an open text")
                        .extend(surfaces(chunk));
                } else {
                    weight += TEXT_WEIGHT;
                    rebuilt.push(surfaces(chunk));
                }
                weight += chunk.iter().map(Weight::weight).sum::<usize>();
            }
            let token = TOKEN_WEIGHT + 2;
            assert!(
                weight < budget + token + TEXT_WEIGHT,
                "part {parts} weighs {weight}, over the budget {budget} by a token and an array or more"
            );
        }
        assert_eq!(rebuilt.len(), len);
        (rebuilt, parts)
    }

    #[test]
    fn flat_parts_give_every_view_once_in_order() {
        for budget in [1, 7, 13, 50, 1_000] {
            let batch = texts(&[3, 0, 5, 1, 0, 0, 12, 2]);
            let expected: Vec<Vec<String>> = batch.iter().map(|text| surfaces(text)).collect();
            let (rebuilt, _) = rebuild(batch, budget);
            assert_eq!(rebuilt, expected, "budget {budget}");
        }
    }

    #[test]
    fn flat_parts_of_budget_one_make_one_token_or_empty_text_each() {
        let sizes = [3, 0, 5, 1, 0, 0, 12, 2];
        let (_, parts) = rebuild(texts(&sizes), 1);
        assert_eq!(parts, sizes.iter().map(|&n| n.max(1)).sum::<usize>());
    }

    #[test]
    fn flat_batch_that_fits_is_one_part() {
        let batch = texts(&[3, 0, 5]);
        // 3 arrays and 8 tokens of 12 each.
        assert!(flat_fits(&batch, 99));
        assert!(!flat_fits(&batch, 98));
        let (_, parts) = rebuild(batch, 99);
        assert_eq!(parts, 1);
    }

    #[test]
    fn nbest_parts_give_every_view_once_in_order() {
        let batch: Vec<NbestViews> = vec![
            vec![
                (texts(&[4]).remove(0), 10),
                (Vec::new(), 11),
                (texts(&[1]).remove(0), 12),
            ],
            Vec::new(),
            vec![(texts(&[7]).remove(0), 20)],
            vec![(texts(&[2]).remove(0), 30), (texts(&[3]).remove(0), 31)],
        ];
        let expected: Vec<Vec<(Vec<String>, i64)>> = batch
            .iter()
            .map(|text| {
                text.iter()
                    .map(|(views, cost)| (surfaces(views), *cost))
                    .collect()
            })
            .collect();
        for budget in [1, 5, 13, 30, 1_000] {
            let mut cursor = NbestCursor::new(batch.clone());
            let mut rebuilt: Vec<Vec<(Vec<String>, i64)>> = Vec::new();
            while let Some(part) = cursor.take(budget) {
                assert_eq!(part.first, rebuilt.len(), "budget {budget}");
                for (index, chunk) in part.chunks.into_iter().enumerate() {
                    let mut results = chunk.into_iter();
                    if index == 0 && part.continues_text {
                        let open = rebuilt.last_mut().expect("an open text");
                        if part.continues_result {
                            let (views, _) = results.next().expect("a continued result");
                            open.last_mut()
                                .expect("an open result")
                                .0
                                .extend(surfaces(&views));
                        }
                        open.extend(results.map(|(views, cost)| (surfaces(&views), cost)));
                    } else {
                        rebuilt.push(
                            results
                                .map(|(views, cost)| (surfaces(&views), cost))
                                .collect(),
                        );
                    }
                }
            }
            assert_eq!(rebuilt, expected, "budget {budget}");
        }
    }
}

//! Making the token objects and the N-best results of every method that
//! returns them (#1409).
//!
//! The objects are those that napi-rs makes of `Token` and `NbestResult`
//! (`crate::token`): plain objects on `Object.prototype`, with the fields in
//! the order of the structs, each writable, enumerable and configurable. They
//! are made with Node-API directly, for four savings over the derived
//! conversion, measured in #1409 on 1.39 million IPADIC tokens (about half
//! of the time of `tokenizeBatch`):
//!
//! * the property names are made once per call, as JavaScript strings,
//!   instead of looked up from C strings for each property of each object;
//! * the values go into handle scopes of a bounded number of tokens, closed
//!   as they fill, instead of the one scope of the call, whose handles every
//!   young-generation collection visits;
//! * each distinct string is made once per scope and used again;
//! * the views, whose strings were allocated on the tokenizer's threads, are
//!   dropped on a thread of their own when there are many.
//!
//! A handle of a closed scope is no longer valid: an array still being
//! filled when a scope closes is read back from its parent, which belongs to
//! the call's scope (`fill`).

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::mpsc;

use lindera_binding::TokenView;
use lindera_binding::tokenizer::NbestViews;
use napi::sys;

use crate::js;

/// The tokens made in one handle scope before it is closed.
pub(crate) const SCOPE_TOKENS: usize = 8_192;

/// The number of tokens from which the views are dropped on a thread of
/// their own.
pub(crate) const DROP_THREAD_TOKENS: usize = 1_024;

/// The fields of a token object, in the order of `Token`.
const TOKEN_FIELDS: [&str; 7] = [
    "surface",
    "byteStart",
    "byteEnd",
    "position",
    "wordId",
    "isUnknown",
    "details",
];

/// The fields of an N-best result, in the order of `NbestResult`.
const NBEST_FIELDS: [&str; 2] = ["tokens", "cost"];

/// The state that the test hooks set (`crate::test_hooks`).
#[cfg(feature = "test-hooks")]
pub(crate) mod hooks {
    use std::sync::atomic::{AtomicU8, AtomicUsize};

    /// The tokens of a scope, or 0 for [`super::SCOPE_TOKENS`].
    pub(crate) static SCOPE: AtomicUsize = AtomicUsize::new(0);

    /// Where the views are dropped: 0 as the number of tokens says, 1 always
    /// on the thread, 2 always on the spot.
    pub(crate) static DROP: AtomicU8 = AtomicU8::new(0);
}

/// The tokens of a scope: [`SCOPE_TOKENS`], or what the test hooks set.
fn scope_tokens() -> usize {
    #[cfg(feature = "test-hooks")]
    {
        let set = hooks::SCOPE.load(std::sync::atomic::Ordering::Relaxed);
        if set > 0 {
            return set;
        }
    }
    SCOPE_TOKENS
}

/// Whether the views of `tokens` tokens go to the thread that drops them.
fn drop_on_thread(tokens: usize) -> bool {
    #[cfg(feature = "test-hooks")]
    match hooks::DROP.load(std::sync::atomic::Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    tokens >= DROP_THREAD_TOKENS
}

/// What the thread that drops the views receives.
type Garbage = Box<dyn Send>;

/// The channel to the thread that drops the views, started on first use, or
/// `None` when the thread could not start.
fn dropper() -> Option<&'static mpsc::Sender<Garbage>> {
    static DROPPER: OnceLock<Option<mpsc::Sender<Garbage>>> = OnceLock::new();
    DROPPER
        .get_or_init(|| {
            let (sender, receiver) = mpsc::channel::<Garbage>();
            std::thread::Builder::new()
                .name("lindera-drop".to_string())
                .spawn(move || {
                    for garbage in receiver {
                        drop(garbage);
                    }
                })
                .ok()
                .map(|_| sender)
        })
        .as_ref()
}

/// Lets the views of a call go once their objects are made: on the thread
/// that drops them when there are many tokens, or else on the spot.
///
/// # Arguments
///
/// * `tokens` - The number of tokens of the views.
/// * `views` - The views.
pub(crate) fn drop_views<T: Send + 'static>(tokens: usize, views: T) {
    if drop_on_thread(tokens)
        && let Some(sender) = dropper()
    {
        if let Err(mpsc::SendError(garbage)) = sender.send(Box::new(views)) {
            drop(garbage);
        }
        return;
    }
    drop(views);
}

/// Makes the objects of one call: the property names, the current handle
/// scope and its table of strings.
///
/// It is made in the scope of the call, where the names stay valid, and
/// opens a scope of its own, which it closes when it is dropped, also on an
/// error or a panic: Node.js stops when a native call returns with a scope
/// left open.
struct Maker<'v> {
    /// The environment of the call.
    env: sys::napi_env,
    /// The names of the fields of a token object.
    token_fields: [sys::napi_value; 7],
    /// The names of the fields of an N-best result.
    nbest_fields: [sys::napi_value; 2],
    /// The strings made in the current scope.
    strings: HashMap<&'v str, sys::napi_value>,
    /// The current scope.
    scope: sys::napi_handle_scope,
    /// The tokens made in the current scope.
    made: usize,
    /// The tokens of a scope.
    bound: usize,
}

impl<'v> Maker<'v> {
    /// Makes the names in the current scope and opens a scope of its own.
    ///
    /// # Safety
    ///
    /// `env` must be the environment of the current call.
    unsafe fn new(env: sys::napi_env) -> napi::Result<Self> {
        let mut token_fields = [std::ptr::null_mut(); 7];
        for (name, field) in token_fields.iter_mut().zip(TOKEN_FIELDS) {
            // SAFETY: the caller guarantees `env`.
            *name = unsafe { js::string(env, field)? };
        }
        let mut nbest_fields = [std::ptr::null_mut(); 2];
        for (name, field) in nbest_fields.iter_mut().zip(NBEST_FIELDS) {
            // SAFETY: as above.
            *name = unsafe { js::string(env, field)? };
        }
        // SAFETY: as above; the maker closes the scope when it is dropped.
        let scope = unsafe { js::open_scope(env)? };
        Ok(Self {
            env,
            token_fields,
            nbest_fields,
            strings: HashMap::new(),
            scope,
            made: 0,
            bound: scope_tokens(),
        })
    }

    /// The string `s`, made once per scope.
    ///
    /// # Safety
    ///
    /// As [`Maker::new`]; the handle is valid until the scope closes.
    unsafe fn string(&mut self, s: &'v str) -> napi::Result<sys::napi_value> {
        if let Some(&value) = self.strings.get(s) {
            return Ok(value);
        }
        // SAFETY: the maker's `env` is the environment of the current call.
        let value = unsafe { js::string(self.env, s)? };
        self.strings.insert(s, value);
        Ok(value)
    }

    /// The object of a token.
    ///
    /// # Safety
    ///
    /// As [`Maker::string`].
    unsafe fn token(&mut self, view: &'v TokenView) -> napi::Result<sys::napi_value> {
        let env = self.env;
        // SAFETY: `env` is the environment of the current call, and every
        // handle belongs to the current scope or to the call's.
        unsafe {
            let details = js::array(env, view.details.len())?;
            for (index, detail) in view.details.iter().enumerate() {
                let value = self.string(detail)?;
                js::set_element(env, details, index as u32, value)?;
            }
            let values = [
                self.string(&view.surface)?,
                js::uint32(env, view.byte_start as u32)?,
                js::uint32(env, view.byte_end as u32)?,
                js::uint32(env, view.position as u32)?,
                js::uint32(env, view.word_id)?,
                js::boolean(env, view.is_unknown)?,
                details,
            ];
            self.made += 1;
            js::object(env, &self.token_fields, values)
        }
    }

    /// The object of an N-best result, with an empty array of `len` tokens.
    ///
    /// # Safety
    ///
    /// As [`Maker::string`].
    unsafe fn nbest_result(&mut self, len: usize, cost: i64) -> napi::Result<sys::napi_value> {
        // SAFETY: as for `token`.
        unsafe {
            let tokens = js::array(self.env, len)?;
            let cost = js::int64(self.env, cost)?;
            self.made += 1;
            js::object(self.env, &self.nbest_fields, [tokens, cost])
        }
    }

    /// Closes the current scope and opens another when it holds its bound of
    /// tokens, and empties the table of strings.
    ///
    /// # Returns
    ///
    /// Whether it did: every handle made since the last scope opened is then
    /// no longer valid.
    ///
    /// # Safety
    ///
    /// As [`Maker::new`]; the maker's scope must be the innermost one.
    unsafe fn renew(&mut self) -> napi::Result<bool> {
        if self.made < self.bound {
            return Ok(false);
        }
        self.strings.clear();
        self.made = 0;
        let scope = std::mem::replace(&mut self.scope, std::ptr::null_mut());
        // SAFETY: the caller guarantees that the maker's scope is the
        // innermost one; a new one replaces it.
        unsafe {
            js::close_scope(self.env, scope)?;
            self.scope = js::open_scope(self.env)?;
        }
        Ok(true)
    }
}

impl Drop for Maker<'_> {
    fn drop(&mut self) {
        if !self.scope.is_null() {
            // SAFETY: the maker lives within one native call, and its scope
            // is the innermost one when it is dropped. A failure leaves
            // nothing more to do.
            let _ = unsafe { js::close_scope(self.env, self.scope) };
        }
    }
}

/// Sets the token objects of `views` into `array` from `position` on.
/// When the maker renews its scope, `array` is read back with `reread`.
///
/// # Returns
///
/// Whether the scope was renewed.
///
/// # Safety
///
/// As [`Maker::renew`]; `array` must be valid, and `reread` must give the
/// same array from handles of an enclosing scope.
unsafe fn fill<'v>(
    maker: &mut Maker<'v>,
    views: &'v [TokenView],
    mut array: sys::napi_value,
    mut position: u32,
    reread: &dyn Fn(sys::napi_env) -> napi::Result<sys::napi_value>,
) -> napi::Result<bool> {
    let env = maker.env;
    let mut renewed = false;
    for view in views {
        // SAFETY: `array` is valid: made in this scope, or read back after
        // the last renewal.
        unsafe {
            let token = maker.token(view)?;
            js::set_element(env, array, position, token)?;
            position += 1;
            if maker.renew()? {
                renewed = true;
                array = reread(env)?;
            }
        }
    }
    Ok(renewed)
}

/// The token objects of a single text: the value of `tokenize`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn tokens(
    env: sys::napi_env,
    views: Vec<TokenView>,
) -> napi::Result<sys::napi_value> {
    // SAFETY: the caller guarantees `env`; the array belongs to the call's
    // scope, so it stays valid as the maker renews its scopes.
    let result = unsafe { js::array(env, views.len())? };
    {
        // SAFETY: as above.
        let mut maker = unsafe { Maker::new(env)? };
        // SAFETY: as above.
        unsafe { fill(&mut maker, &views, result, 0, &|_| Ok(result))? };
    }
    drop_views(views.len(), views);
    Ok(result)
}

/// Sets the token objects of `chunks` into `outer`: each chunk starts the
/// text at the next index from `first`, except the first one when
/// `continues`, whose tokens go after those of the text before `first`.
/// A batch at once is the chunks of all its texts from 0.
///
/// # Safety
///
/// `env` must be the environment of the current call, and `outer` an array
/// handle of it (of the call's scope).
pub(crate) unsafe fn fill_texts(
    env: sys::napi_env,
    outer: sys::napi_value,
    first: u32,
    continues: bool,
    chunks: &[Vec<TokenView>],
) -> napi::Result<()> {
    // SAFETY: the caller guarantees `env` and `outer`; every text's array is
    // in `outer` before it is filled, so it can be read back from it.
    unsafe {
        let mut maker = Maker::new(env)?;
        let mut next = first;
        for (index, chunk) in chunks.iter().enumerate() {
            let (text, array, position) = if index == 0 && continues {
                let text = next - 1;
                let array = js::element(env, outer, text)?;
                (text, array, js::length(env, array)?)
            } else {
                let array = js::array(env, chunk.len())?;
                js::set_element(env, outer, next, array)?;
                next += 1;
                (next - 1, array, 0)
            };
            fill(&mut maker, chunk, array, position, &|env| {
                js::element(env, outer, text)
            })?;
        }
    }
    Ok(())
}

/// The token objects of a batch at once: the value of `tokenizeBatch`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn token_batch(
    env: sys::napi_env,
    batch: Vec<Vec<TokenView>>,
) -> napi::Result<sys::napi_value> {
    // SAFETY: the caller guarantees `env`; `outer` belongs to the call's
    // scope.
    let outer = unsafe { js::array(env, batch.len())? };
    // SAFETY: as above.
    unsafe { fill_texts(env, outer, 0, false, &batch)? };
    let tokens = batch.iter().map(Vec::len).sum();
    drop_views(tokens, batch);
    Ok(outer)
}

/// Sets the N-best results of `results` into the array that `reread` gives
/// (`target` until the scope is renewed), from `start` on.
///
/// # Safety
///
/// As [`fill`].
unsafe fn fill_results<'v>(
    maker: &mut Maker<'v>,
    results: &'v [(Vec<TokenView>, i64)],
    mut target: sys::napi_value,
    start: u32,
    reread: &dyn Fn(sys::napi_env) -> napi::Result<sys::napi_value>,
) -> napi::Result<()> {
    let env = maker.env;
    let tokens_field = maker.nbest_fields[0];
    for (offset, (views, cost)) in results.iter().enumerate() {
        let index = start + offset as u32;
        // SAFETY: `target` is valid: made in this scope, or read back after
        // the last renewal; the result is in `target` before it is filled.
        unsafe {
            let result = maker.nbest_result(views.len(), *cost)?;
            js::set_element(env, target, index, result)?;
            let tokens = js::property(env, result, tokens_field)?;
            let renewed = fill(maker, views, tokens, 0, &|env| {
                js::property(env, js::element(env, reread(env)?, index)?, tokens_field)
            })?;
            if renewed || maker.renew()? {
                target = reread(env)?;
            }
        }
    }
    Ok(())
}

/// The N-best results of a single text: the value of `tokenizeNbest`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn nbest(
    env: sys::napi_env,
    results: NbestViews,
) -> napi::Result<sys::napi_value> {
    // SAFETY: the caller guarantees `env`; the array belongs to the call's
    // scope.
    let target = unsafe { js::array(env, results.len())? };
    {
        // SAFETY: as above.
        let mut maker = unsafe { Maker::new(env)? };
        // SAFETY: as above.
        unsafe { fill_results(&mut maker, &results, target, 0, &|_| Ok(target))? };
    }
    let tokens = results.iter().map(|(views, _)| views.len()).sum();
    drop_views(tokens, results);
    Ok(target)
}

/// Sets the N-best results of `chunks` into `outer`, as [`fill_texts`] does
/// for tokens. With `continues_result`, the first result of the first chunk
/// continues the last result of the text before `first`: its tokens go
/// after that result's, and its cost is not used.
///
/// # Safety
///
/// As [`fill_texts`].
pub(crate) unsafe fn fill_nbest_texts(
    env: sys::napi_env,
    outer: sys::napi_value,
    first: u32,
    continues_text: bool,
    continues_result: bool,
    chunks: &[NbestViews],
) -> napi::Result<()> {
    // SAFETY: the caller guarantees `env` and `outer`; every array is in its
    // parent before it is filled, so it can be read back from `outer`.
    unsafe {
        let mut maker = Maker::new(env)?;
        let tokens_field = maker.nbest_fields[0];
        let mut next = first;
        for (index, chunk) in chunks.iter().enumerate() {
            if index > 0 || !continues_text {
                let target = js::array(env, chunk.len())?;
                js::set_element(env, outer, next, target)?;
                let text = next;
                next += 1;
                fill_results(&mut maker, chunk, target, 0, &|env| {
                    js::element(env, outer, text)
                })?;
                continue;
            }
            let text = next - 1;
            let target = js::element(env, outer, text)?;
            let mut start = js::length(env, target)?;
            let mut results = &chunk[..];
            if continues_result && let Some(((views, _), rest)) = chunk.split_first() {
                let last = start - 1;
                let tokens = js::property(env, js::element(env, target, last)?, tokens_field)?;
                let position = js::length(env, tokens)?;
                fill(&mut maker, views, tokens, position, &|env| {
                    js::property(
                        env,
                        js::element(env, js::element(env, outer, text)?, last)?,
                        tokens_field,
                    )
                })?;
                results = rest;
                start = last + 1;
            }
            let target = js::element(env, outer, text)?;
            fill_results(&mut maker, results, target, start, &|env| {
                js::element(env, outer, text)
            })?;
        }
    }
    Ok(())
}

/// The N-best results of a batch at once: the value of
/// `tokenizeNbestBatch`.
///
/// # Safety
///
/// `env` must be the environment of the current call.
pub(crate) unsafe fn nbest_batch(
    env: sys::napi_env,
    batch: Vec<NbestViews>,
) -> napi::Result<sys::napi_value> {
    // SAFETY: the caller guarantees `env`; `outer` belongs to the call's
    // scope.
    let outer = unsafe { js::array(env, batch.len())? };
    // SAFETY: as above.
    unsafe { fill_nbest_texts(env, outer, 0, false, false, &batch)? };
    let tokens = batch
        .iter()
        .flat_map(|text| text.iter().map(|(views, _)| views.len()))
        .sum();
    drop_views(tokens, batch);
    Ok(outer)
}

//! # Lindera Node.js Bindings
//!
//! Node.js bindings for [Lindera](https://github.com/lindera/lindera), a morphological analysis library for CJK text.
//!
//! Built with [NAPI-RS](https://napi.rs/) for high-performance native Node.js addon support.
//!
//! ## Features
//!
//! - **Dictionary management**: Build, load, and use custom dictionaries
//! - **Tokenization**: Multiple tokenization modes (normal, decompose)
//! - **Filters**: Character and token filtering pipeline
//! - **Training**: Train custom morphological models (with `train` feature)
//! - **User dictionaries**: Support for custom user dictionaries
//! - **TypeScript**: Full type definitions generated automatically
//!
//! ## Panics
//!
//! Every exported function and method is marked `#[napi(catch_unwind)]`, so
//! that a panic in Rust throws a JavaScript `Error` (`code` `GenericFailure`,
//! the panic's message as `message`) from the function or method called,
//! instead of unwinding into napi-rs's `extern "C"` callback and aborting
//! Node.js (#1314). A unit test checks that none lacks it.

#[macro_use]
extern crate napi_derive;

pub mod dictionary;
pub mod error;
pub mod metadata;
pub mod mode;
pub mod schema;
pub mod token;
pub mod tokenizer;
pub mod util;

#[cfg(feature = "test-hooks")]
pub mod test_hooks;

#[cfg(feature = "train")]
pub mod trainer;

/// Returns the version of the lindera-nodejs package.
#[napi(catch_unwind)]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    use syn::{Attribute, ImplItem, Item, Meta, Type};

    /// Tells whether `attrs` hold a `#[napi]` attribute and, if so, whether
    /// one of them has `catch_unwind`.
    ///
    /// # Arguments
    ///
    /// * `attrs` - The attributes of a function or method.
    ///
    /// # Returns
    ///
    /// `None` without `#[napi]`, otherwise `Some(true)` when a `#[napi(...)]`
    /// lists `catch_unwind` and `Some(false)` when none does.
    fn catches_panics(attrs: &[Attribute]) -> Option<bool> {
        let mut is_napi = false;
        let mut catches = false;
        for attr in attrs.iter().filter(|attr| attr.path().is_ident("napi")) {
            is_napi = true;
            if let Meta::List(_) = attr.meta {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("catch_unwind") {
                        catches = true;
                    }
                    // Skip the value of `js_name = "..."` and the like.
                    if meta.input.peek(syn::Token![=]) {
                        meta.value()?.parse::<syn::Expr>()?;
                    }
                    Ok(())
                })
                .unwrap_or_else(|err| panic!("cannot read a #[napi] attribute: {err}"));
            }
        }
        is_napi.then_some(catches)
    }

    /// Records `name` in `found` when `attrs` hold `#[napi]`, and in
    /// `missing` too when none of them has `catch_unwind`.
    ///
    /// # Arguments
    ///
    /// * `name` - The function or method, as `file::name` or
    ///   `file::Type::name`.
    /// * `attrs` - Its attributes.
    /// * `found` - Receives every `#[napi]` function and method.
    /// * `missing` - Receives those without `catch_unwind`.
    fn check(
        name: String,
        attrs: &[Attribute],
        found: &mut BTreeSet<String>,
        missing: &mut Vec<String>,
    ) {
        if let Some(catches) = catches_panics(attrs) {
            if !catches {
                missing.push(name.clone());
            }
            found.insert(name);
        }
    }

    /// Walks `items` and records each `#[napi]` function and method with
    /// [`check`].
    ///
    /// # Arguments
    ///
    /// * `file` - The name of the source file, for the records.
    /// * `items` - The items of the file, or of an inline module in it.
    /// * `found` - Receives every `#[napi]` function and method.
    /// * `missing` - Receives those without `catch_unwind`.
    fn collect(
        file: &str,
        items: &[Item],
        found: &mut BTreeSet<String>,
        missing: &mut Vec<String>,
    ) {
        for item in items {
            match item {
                Item::Fn(function) => check(
                    format!("{file}::{}", function.sig.ident),
                    &function.attrs,
                    found,
                    missing,
                ),
                Item::Impl(block) => {
                    let ty = match &*block.self_ty {
                        Type::Path(path) => path
                            .path
                            .segments
                            .last()
                            .map_or_else(String::new, |segment| segment.ident.to_string()),
                        _ => String::new(),
                    };
                    for impl_item in &block.items {
                        if let ImplItem::Fn(method) = impl_item {
                            check(
                                format!("{file}::{ty}::{}", method.sig.ident),
                                &method.attrs,
                                found,
                                missing,
                            );
                        }
                    }
                }
                Item::Mod(module) => {
                    if let Some((_, items)) = &module.content {
                        collect(file, items, found, missing);
                    }
                }
                _ => {}
            }
        }
    }

    /// Every `#[napi]` function and method of the crate must carry
    /// `catch_unwind`: without it, a panic in it unwinds into napi-rs's
    /// `extern "C"` callback and aborts Node.js with `fatal runtime error:
    /// failed to initiate panic, error 5, aborting` instead of throwing
    /// (#1314).
    #[test]
    fn every_napi_function_catches_panics() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut paths: Vec<_> = fs::read_dir(&src)
            .unwrap_or_else(|err| panic!("cannot list {}: {err}", src.display()))
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<_, _>>()
            .unwrap_or_else(|err| panic!("cannot list {}: {err}", src.display()));
        paths.retain(|path| path.extension().is_some_and(|ext| ext == "rs"));
        paths.sort();

        let mut found = BTreeSet::new();
        let mut missing = Vec::new();
        for path in &paths {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
            let parsed = syn::parse_file(&source)
                .unwrap_or_else(|err| panic!("cannot parse {}: {err}", path.display()));
            let file = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            collect(&file, &parsed.items, &mut found, &mut missing);
        }

        // A walk that finds nothing must not pass: these are a free
        // function, a method, a `&mut self` method and a getter.
        for expected in [
            "lib.rs::version",
            "tokenizer.rs::JsTokenizer::tokenize",
            "tokenizer.rs::JsTokenizerBuilder::set_mode",
            "metadata.rs::JsMetadata::default_word_cost",
        ] {
            assert!(
                found.contains(expected),
                "{expected} is not among the #[napi] functions found: {found:?}"
            );
        }
        assert!(
            missing.is_empty(),
            "#[napi] functions without catch_unwind, whose panics abort Node.js: {missing:?}"
        );
    }
}

//! Nesting limit for argument values that the bindings convert to JSON.
//!
//! `TokenizerBuilder.setSpacePenalty`, `appendCharacterFilter` and
//! `appendTokenFilter` take a host-language value (a JS object, a Python
//! dict, a Ruby hash, a PHP array) and convert it to a `serde_json::Value`
//! by recursing into every container. Without a limit, a value that contains
//! itself, or one nested a few thousand levels deep, overflows the native
//! stack and takes the host process down. Every binding counts the nesting
//! while it converts and stops at [`MAX_ARGUMENT_DEPTH`], so such a value
//! becomes an ordinary, catchable error instead.

/// Deepest container nesting a converted argument may have.
///
/// Containers (arrays, objects, dicts, hashes) count; the outermost one is
/// level 1, and a container at level `MAX_ARGUMENT_DEPTH + 1` is rejected.
/// A value that contains itself is infinitely deep, so this limit also stops
/// cycles. Real configuration objects are a few levels deep.
pub const MAX_ARGUMENT_DEPTH: usize = 128;

/// Returns the error message for an argument deeper than
/// [`MAX_ARGUMENT_DEPTH`].
///
/// # Returns
///
/// The message every binding raises, so the error reads the same in each
/// language.
pub fn argument_too_deep_message() -> String {
    format!(
        "argument is nested more than {MAX_ARGUMENT_DEPTH} levels deep \
         (a value that contains itself is always too deep)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_too_deep_message_names_the_limit() {
        let message = argument_too_deep_message();
        assert!(message.contains("nested more than 128 levels deep"));
        assert!(message.contains("contains itself"));
    }
}

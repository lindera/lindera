//! Segmenter implementation for morphological analysis in PHP.

/// Core segmenter for morphological analysis.
///
/// This is mostly used internally by the Tokenizer.
pub struct PhpSegmenter {
    /// The inner Lindera segmenter.
    pub inner: lindera::segmenter::Segmenter,
}

//! Segmenter implementation for morphological analysis.

/// Core segmenter for morphological analysis (internal use).
#[derive(Clone)]
pub struct RbSegmenter {
    /// Inner Lindera segmenter.
    pub inner: lindera::segmenter::Segmenter,
}

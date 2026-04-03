use wasm_bindgen::prelude::*;

use lindera::dictionary::{CompressionAlgorithm, Metadata};

use crate::schema::JsSchema;

/// Compression algorithm for dictionary data.
#[wasm_bindgen(js_name = "CompressionAlgorithm")]
#[derive(Clone, Copy)]
pub enum JsCompressionAlgorithm {
    Deflate,
    Zlib,
    Gzip,
    Raw,
}

impl From<CompressionAlgorithm> for JsCompressionAlgorithm {
    fn from(alg: CompressionAlgorithm) -> Self {
        match alg {
            CompressionAlgorithm::Deflate => JsCompressionAlgorithm::Deflate,
            CompressionAlgorithm::Zlib => JsCompressionAlgorithm::Zlib,
            CompressionAlgorithm::Gzip => JsCompressionAlgorithm::Gzip,
            CompressionAlgorithm::Raw => JsCompressionAlgorithm::Raw,
        }
    }
}

impl From<JsCompressionAlgorithm> for CompressionAlgorithm {
    fn from(alg: JsCompressionAlgorithm) -> Self {
        match alg {
            JsCompressionAlgorithm::Deflate => CompressionAlgorithm::Deflate,
            JsCompressionAlgorithm::Zlib => CompressionAlgorithm::Zlib,
            JsCompressionAlgorithm::Gzip => CompressionAlgorithm::Gzip,
            JsCompressionAlgorithm::Raw => CompressionAlgorithm::Raw,
        }
    }
}

/// Dictionary metadata configuration.
#[wasm_bindgen(js_name = "Metadata")]
#[derive(Clone)]
pub struct JsMetadata {
    pub(crate) inner: Metadata,
}

#[wasm_bindgen]
impl JsMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: Option<String>,
        encoding: Option<String>,
        compress_algorithm: Option<JsCompressionAlgorithm>,
    ) -> Self {
        let mut inner = Metadata::default();
        if let Some(n) = name {
            inner.name = n;
        }
        if let Some(e) = encoding {
            inner.encoding = e;
        }
        if let Some(a) = compress_algorithm {
            inner.compress_algorithm = a.into();
        }
        Self { inner }
    }

    #[wasm_bindgen(js_name = "createDefault")]
    pub fn create_default() -> Self {
        Self {
            inner: Metadata::default(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    #[wasm_bindgen(getter)]
    pub fn encoding(&self) -> String {
        self.inner.encoding.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_encoding(&mut self, encoding: String) {
        self.inner.encoding = encoding;
    }

    #[wasm_bindgen(getter)]
    pub fn compress_algorithm(&self) -> JsCompressionAlgorithm {
        self.inner.compress_algorithm.into()
    }

    #[wasm_bindgen(setter)]
    pub fn set_compress_algorithm(&mut self, algorithm: JsCompressionAlgorithm) {
        self.inner.compress_algorithm = algorithm.into();
    }

    #[wasm_bindgen(getter)]
    pub fn dictionary_schema(&self) -> JsSchema {
        JsSchema {
            inner: self.inner.dictionary_schema.clone(),
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_dictionary_schema(&mut self, schema: JsSchema) {
        self.inner.dictionary_schema = schema.inner;
    }

    #[wasm_bindgen(getter)]
    pub fn user_dictionary_schema(&self) -> JsSchema {
        JsSchema {
            inner: self.inner.user_dictionary_schema.clone(),
        }
    }

    #[wasm_bindgen(setter)]
    pub fn set_user_dictionary_schema(&mut self, schema: JsSchema) {
        self.inner.user_dictionary_schema = schema.inner;
    }
}

impl From<Metadata> for JsMetadata {
    fn from(metadata: Metadata) -> Self {
        JsMetadata { inner: metadata }
    }
}

impl From<JsMetadata> for Metadata {
    fn from(metadata: JsMetadata) -> Self {
        metadata.inner
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_new() {
        let metadata = JsMetadata::new(
            Some("test".to_string()),
            Some("utf-8".to_string()),
            Some(JsCompressionAlgorithm::Deflate),
        );

        assert_eq!(metadata.name(), "test");
        assert_eq!(metadata.encoding(), "utf-8");
        assert!(matches!(
            metadata.compress_algorithm(),
            JsCompressionAlgorithm::Deflate
        ));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_create_default() {
        let metadata = JsMetadata::create_default();

        // Default values should not be empty
        assert!(!metadata.name().is_empty());
        assert!(!metadata.encoding().is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_setters() {
        let mut metadata = JsMetadata::create_default();

        metadata.set_name("custom_name".to_string());
        assert_eq!(metadata.name(), "custom_name");

        metadata.set_encoding("euc-jp".to_string());
        assert_eq!(metadata.encoding(), "euc-jp");

        metadata.set_compress_algorithm(JsCompressionAlgorithm::Gzip);
        assert!(matches!(
            metadata.compress_algorithm(),
            JsCompressionAlgorithm::Gzip
        ));
    }
}

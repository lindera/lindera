use wasm_bindgen::prelude::*;

use lindera::dictionary::Metadata;

use crate::schema::JsSchema;

/// Dictionary metadata configuration.
#[wasm_bindgen(js_name = "Metadata")]
#[derive(Clone)]
pub struct JsMetadata {
    pub(crate) inner: Metadata,
}

#[wasm_bindgen]
impl JsMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(name: Option<String>, encoding: Option<String>) -> Self {
        let mut inner = Metadata::default();
        if let Some(n) = name {
            inner.name = n;
        }
        if let Some(e) = encoding {
            inner.encoding = e;
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
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_new_wasm() {
        let metadata = JsMetadata::new(Some("test".to_string()), Some("utf-8".to_string()));

        assert_eq!(metadata.name(), "test");
        assert_eq!(metadata.encoding(), "utf-8");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_create_default_wasm() {
        let metadata = JsMetadata::create_default();

        // Default values should not be empty
        assert!(!metadata.name().is_empty());
        assert!(!metadata.encoding().is_empty());
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_metadata_setters_wasm() {
        let mut metadata = JsMetadata::create_default();

        metadata.set_name("custom_name".to_string());
        assert_eq!(metadata.name(), "custom_name");

        metadata.set_encoding("euc-jp".to_string());
        assert_eq!(metadata.encoding(), "euc-jp");
    }

    #[test]
    fn test_metadata_new() {
        let metadata = JsMetadata::new(Some("test".to_string()), Some("utf-8".to_string()));

        assert_eq!(metadata.name(), "test");
        assert_eq!(metadata.encoding(), "utf-8");
    }

    #[test]
    fn test_metadata_new_with_defaults() {
        let metadata = JsMetadata::new(None, None);

        assert_eq!(metadata.name(), "default");
        assert_eq!(metadata.encoding(), "UTF-8");
    }

    #[test]
    fn test_metadata_create_default() {
        let metadata = JsMetadata::create_default();

        assert_eq!(metadata.name(), "default");
        assert_eq!(metadata.encoding(), "UTF-8");

        let dict_schema = metadata.dictionary_schema();
        assert_eq!(dict_schema.field_count(), 13);
    }

    #[test]
    fn test_metadata_setters() {
        let mut metadata = JsMetadata::create_default();

        metadata.set_name("custom_name".to_string());
        assert_eq!(metadata.name(), "custom_name");

        metadata.set_encoding("euc-jp".to_string());
        assert_eq!(metadata.encoding(), "euc-jp");
    }

    #[test]
    fn test_metadata_schema_setters() {
        let mut metadata = JsMetadata::create_default();

        let custom_schema = JsSchema::new(vec!["surface".to_string(), "reading".to_string()]);
        metadata.set_dictionary_schema(custom_schema);
        assert_eq!(metadata.dictionary_schema().field_count(), 2);

        let user_schema = JsSchema::new(vec![
            "surface".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ]);
        metadata.set_user_dictionary_schema(user_schema);
        assert_eq!(metadata.user_dictionary_schema().field_count(), 3);
    }

    #[test]
    fn test_metadata_from_into_conversions() {
        let js_metadata = JsMetadata::new(Some("test_dict".to_string()), Some("utf-8".to_string()));

        let lindera_metadata: Metadata = js_metadata.into();
        assert_eq!(lindera_metadata.name, "test_dict");
        assert_eq!(lindera_metadata.encoding, "utf-8");

        let back: JsMetadata = lindera_metadata.into();
        assert_eq!(back.name(), "test_dict");
        assert_eq!(back.encoding(), "utf-8");
    }
}

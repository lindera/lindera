use wasm_bindgen::prelude::*;

use lindera::dictionary::Metadata;
use lindera_binding::CoreMetadata;

use crate::schema::JsSchema;

/// Dictionary metadata configuration.
///
/// A thin wasm-bindgen wrapper over [`lindera_binding::CoreMetadata`], which
/// owns the default values and the schema wiring.
#[wasm_bindgen(js_name = "Metadata")]
#[derive(Clone)]
pub struct JsMetadata {
    /// The backing lindera-binding metadata.
    pub(crate) inner: CoreMetadata,
}

#[wasm_bindgen(js_class = "Metadata")]
impl JsMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(name: Option<String>, encoding: Option<String>) -> Self {
        Self {
            inner: CoreMetadata::new(
                name, encoding, None, None, None, None, None, None, None, None,
            ),
        }
    }

    #[wasm_bindgen(js_name = "createDefault")]
    pub fn create_default() -> Self {
        Self {
            inner: CoreMetadata::create_default(),
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
        JsSchema::from(self.inner.dictionary_schema.clone())
    }

    /// Sets the schema of the system dictionary to a copy of `schema`.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema. It is borrowed, so it stays usable and can be
    ///   set again (#1238).
    #[wasm_bindgen(setter)]
    pub fn set_dictionary_schema(&mut self, schema: &JsSchema) {
        self.inner.dictionary_schema = schema.inner.clone();
    }

    #[wasm_bindgen(getter)]
    pub fn user_dictionary_schema(&self) -> JsSchema {
        JsSchema::from(self.inner.user_dictionary_schema.clone())
    }

    /// Sets the schema of user dictionaries to a copy of `schema`.
    ///
    /// # Arguments
    ///
    /// * `schema` - The schema. It is borrowed, so it stays usable and can be
    ///   set again (#1238).
    #[wasm_bindgen(setter)]
    pub fn set_user_dictionary_schema(&mut self, schema: &JsSchema) {
        self.inner.user_dictionary_schema = schema.inner.clone();
    }
}

impl From<Metadata> for JsMetadata {
    fn from(metadata: Metadata) -> Self {
        JsMetadata {
            inner: CoreMetadata::from(metadata),
        }
    }
}

impl From<JsMetadata> for Metadata {
    fn from(metadata: JsMetadata) -> Self {
        metadata.inner.into()
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
        metadata.set_dictionary_schema(&custom_schema);
        assert_eq!(metadata.dictionary_schema().field_count(), 2);

        let user_schema = JsSchema::new(vec![
            "surface".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ]);
        metadata.set_user_dictionary_schema(&user_schema);
        assert_eq!(metadata.user_dictionary_schema().field_count(), 3);
    }

    /// The schema setters leave their schema usable, so one schema can be
    /// set on several `Metadata` (#1238). Set through JS: the JS glue, which
    /// a call from Rust skips, destroyed a schema taken by value.
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_schema_setters_leave_the_schema_usable() {
        use crate::test_utils::call_method;

        let field_count = |schema: Result<JsValue, JsValue>| {
            schema.and_then(|schema| call_method(&schema, "field_count", &[]))
        };
        let mut failures = Vec::new();
        for property in ["dictionary_schema", "user_dictionary_schema"] {
            let schema = JsValue::from(JsSchema::new(vec![
                "surface".to_string(),
                "reading".to_string(),
            ]));
            for attempt in 0..2 {
                let metadata = JsValue::from(JsMetadata::create_default());
                let set = js_sys::Reflect::set(&metadata, &property.into(), &schema)
                    .and_then(|_| js_sys::Reflect::get(&metadata, &property.into()));
                let count = field_count(set);
                if count.as_ref().ok().and_then(JsValue::as_f64) != Some(2.0) {
                    failures.push(format!("{property}, attempt {attempt}: {count:?}"));
                }
            }
            let count = field_count(Ok(schema));
            if count.as_ref().ok().and_then(JsValue::as_f64) != Some(2.0) {
                failures.push(format!("the schema after {property}: {count:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
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

    /// ko-dic's `metadata.json` ships left-space penalty rules. Loaded the
    /// way `loadDictionaryFromBytes()` does, they must survive the
    /// `dictionary.metadata` wrapper round trip (#1052), even though the
    /// WASM `Metadata` class does not expose them as a property.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_metadata_keeps_ko_dic_space_penalty() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../lindera-ko-dic/metadata.json");
        let bytes = std::fs::read(&path).unwrap();
        let loaded = Metadata::load(&bytes).unwrap();
        let shipped = loaded.space_penalty.clone().unwrap();

        let wrapped = JsMetadata::from(loaded);
        let as_json = wrapped.inner.space_penalty_json().unwrap();
        assert!(as_json.contains("JKS"), "{as_json}");

        let back: Metadata = wrapped.into();
        assert_eq!(back.space_penalty, Some(shipped));
    }
}

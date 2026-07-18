use wasm_bindgen::prelude::*;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};
use lindera_binding_core::{CoreFieldDefinition, CoreFieldType, CoreSchema};

/// Field type in dictionary schema.
#[wasm_bindgen(js_name = "FieldType")]
#[derive(Debug, Clone, Copy)]
pub enum JsFieldType {
    /// Surface form (word text)
    Surface,
    /// Left context ID for morphological analysis
    LeftContextId,
    /// Right context ID for morphological analysis
    RightContextId,
    /// Word cost (used in path selection)
    Cost,
    /// Custom field (morphological features)
    Custom,
}

impl From<CoreFieldType> for JsFieldType {
    fn from(field_type: CoreFieldType) -> Self {
        match field_type {
            CoreFieldType::Surface => JsFieldType::Surface,
            CoreFieldType::LeftContextId => JsFieldType::LeftContextId,
            CoreFieldType::RightContextId => JsFieldType::RightContextId,
            CoreFieldType::Cost => JsFieldType::Cost,
            CoreFieldType::Custom => JsFieldType::Custom,
        }
    }
}

impl From<JsFieldType> for CoreFieldType {
    fn from(field_type: JsFieldType) -> Self {
        match field_type {
            JsFieldType::Surface => CoreFieldType::Surface,
            JsFieldType::LeftContextId => CoreFieldType::LeftContextId,
            JsFieldType::RightContextId => CoreFieldType::RightContextId,
            JsFieldType::Cost => CoreFieldType::Cost,
            JsFieldType::Custom => CoreFieldType::Custom,
        }
    }
}

impl From<FieldType> for JsFieldType {
    fn from(field_type: FieldType) -> Self {
        JsFieldType::from(CoreFieldType::from(field_type))
    }
}

impl From<JsFieldType> for FieldType {
    fn from(field_type: JsFieldType) -> Self {
        FieldType::from(CoreFieldType::from(field_type))
    }
}

/// Field definition in dictionary schema.
#[wasm_bindgen(js_name = "FieldDefinition")]
#[derive(Clone)]
pub struct JsFieldDefinition {
    pub index: usize,
    #[wasm_bindgen(getter_with_clone)]
    pub name: String,
    pub field_type: JsFieldType,
    #[wasm_bindgen(getter_with_clone)]
    pub description: Option<String>,
}

#[wasm_bindgen(js_class = "FieldDefinition")]
impl JsFieldDefinition {
    #[wasm_bindgen(constructor)]
    pub fn new(
        index: usize,
        name: String,
        field_type: JsFieldType,
        description: Option<String>,
    ) -> Self {
        Self {
            index,
            name,
            field_type,
            description,
        }
    }
}

impl From<CoreFieldDefinition> for JsFieldDefinition {
    fn from(field_def: CoreFieldDefinition) -> Self {
        JsFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<JsFieldDefinition> for CoreFieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        CoreFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<FieldDefinition> for JsFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        JsFieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

impl From<JsFieldDefinition> for FieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        FieldDefinition::from(CoreFieldDefinition::from(field_def))
    }
}

/// Dictionary schema definition.
///
/// A thin wasm-bindgen wrapper over [`lindera_binding_core::CoreSchema`], which
/// owns the field storage, the name-to-index map, and the field lookups.
#[wasm_bindgen(js_name = "Schema")]
#[derive(Clone)]
pub struct JsSchema {
    /// The backing binding-core schema.
    pub(crate) inner: CoreSchema,
}

#[wasm_bindgen(js_class = "Schema")]
impl JsSchema {
    #[wasm_bindgen(constructor)]
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            inner: CoreSchema::new(fields),
        }
    }

    pub fn create_default() -> Self {
        Self {
            inner: CoreSchema::create_default(),
        }
    }

    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.inner.get_field_index(field_name)
    }

    pub fn field_count(&self) -> usize {
        self.inner.field_count()
    }

    pub fn get_field_name(&self, index: usize) -> Option<String> {
        self.inner.get_field_name(index).map(str::to_string)
    }

    pub fn get_custom_fields(&self) -> Vec<String> {
        self.inner.get_custom_fields().to_vec()
    }

    pub fn get_all_fields(&self) -> Vec<String> {
        self.inner.fields().to_vec()
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<JsFieldDefinition> {
        self.inner
            .get_field_by_name(name)
            .map(JsFieldDefinition::from)
    }
}

impl From<CoreSchema> for JsSchema {
    fn from(schema: CoreSchema) -> Self {
        JsSchema { inner: schema }
    }
}

impl From<JsSchema> for CoreSchema {
    fn from(schema: JsSchema) -> Self {
        schema.inner
    }
}

impl From<Schema> for JsSchema {
    fn from(schema: Schema) -> Self {
        JsSchema {
            inner: CoreSchema::from(schema),
        }
    }
}

impl From<JsSchema> for Schema {
    fn from(schema: JsSchema) -> Self {
        schema.inner.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_schema_new_wasm() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let schema = JsSchema::new(fields);

        assert_eq!(schema.field_count(), 5);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_schema_field_operations_wasm() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
            "reading".to_string(),
        ];
        let schema = JsSchema::new(fields.clone());

        assert_eq!(schema.field_count(), 6);
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("pos"), Some(4));
        assert_eq!(schema.get_field_index("nonexistent"), None);
        assert_eq!(schema.get_field_name(0), Some("surface".to_string()));
        assert_eq!(schema.get_field_name(999), None);
        assert_eq!(schema.get_all_fields(), fields);
        let custom = schema.get_custom_fields();
        assert_eq!(custom, vec!["pos".to_string(), "reading".to_string()]);
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_schema_get_field_by_name_wasm() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let schema = JsSchema::new(fields);

        let surface_field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface_field.index, 0);
        assert_eq!(surface_field.name, "surface");
        assert!(matches!(surface_field.field_type, JsFieldType::Surface));

        let pos_field = schema.get_field_by_name("pos").unwrap();
        assert_eq!(pos_field.index, 4);
        assert!(matches!(pos_field.field_type, JsFieldType::Custom));

        assert!(schema.get_field_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_schema_new() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let schema = JsSchema::new(fields);

        assert_eq!(schema.field_count(), 5);
    }

    #[test]
    fn test_schema_field_operations() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
            "reading".to_string(),
        ];
        let schema = JsSchema::new(fields.clone());

        assert_eq!(schema.field_count(), 6);

        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("pos"), Some(4));
        assert_eq!(schema.get_field_index("nonexistent"), None);

        assert_eq!(schema.get_field_name(0), Some("surface".to_string()));
        assert_eq!(schema.get_field_name(999), None);

        let all = schema.get_all_fields();
        assert_eq!(all, fields);

        let custom = schema.get_custom_fields();
        assert_eq!(custom, vec!["pos".to_string(), "reading".to_string()]);
    }

    #[test]
    fn test_schema_get_custom_fields_no_custom() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
        ];
        let schema = JsSchema::new(fields);

        assert!(schema.get_custom_fields().is_empty());
    }

    #[test]
    fn test_schema_create_default() {
        let schema = JsSchema::create_default();

        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("left_context_id"), Some(1));
        assert_eq!(schema.get_field_index("right_context_id"), Some(2));
        assert_eq!(schema.get_field_index("cost"), Some(3));
        assert_eq!(schema.get_field_index("major_pos"), Some(4));
        assert_eq!(schema.get_field_index("pos_detail_1"), Some(5));
        assert_eq!(schema.get_field_index("pronunciation"), Some(12));
    }

    #[test]
    fn test_schema_get_field_by_name() {
        let fields = vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
            "pos".to_string(),
        ];
        let schema = JsSchema::new(fields);

        let surface_field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface_field.index, 0);
        assert_eq!(surface_field.name, "surface");
        assert!(matches!(surface_field.field_type, JsFieldType::Surface));

        let left_id_field = schema.get_field_by_name("left_id").unwrap();
        assert_eq!(left_id_field.index, 1);
        assert!(matches!(
            left_id_field.field_type,
            JsFieldType::LeftContextId
        ));

        let cost_field = schema.get_field_by_name("cost").unwrap();
        assert_eq!(cost_field.index, 3);
        assert!(matches!(cost_field.field_type, JsFieldType::Cost));

        let pos_field = schema.get_field_by_name("pos").unwrap();
        assert_eq!(pos_field.index, 4);
        assert!(matches!(pos_field.field_type, JsFieldType::Custom));

        assert!(schema.get_field_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_field_type_from_into_conversions() {
        let pairs = [
            (JsFieldType::Surface, FieldType::Surface),
            (JsFieldType::LeftContextId, FieldType::LeftContextId),
            (JsFieldType::RightContextId, FieldType::RightContextId),
            (JsFieldType::Cost, FieldType::Cost),
            (JsFieldType::Custom, FieldType::Custom),
        ];

        for (js_type, lindera_type) in pairs {
            let converted: FieldType = js_type.into();
            assert_eq!(
                std::mem::discriminant(&converted),
                std::mem::discriminant(&lindera_type)
            );

            let back: JsFieldType = lindera_type.into();
            assert_eq!(
                std::mem::discriminant(&back),
                std::mem::discriminant(&js_type)
            );
        }
    }

    #[test]
    fn test_field_definition_from_into_conversions() {
        let js_field = JsFieldDefinition::new(
            4,
            "pos".to_string(),
            JsFieldType::Custom,
            Some("Part of speech".to_string()),
        );

        let lindera_field: FieldDefinition = js_field.into();
        assert_eq!(lindera_field.index, 4);
        assert_eq!(lindera_field.name, "pos");
        assert!(matches!(lindera_field.field_type, FieldType::Custom));

        let back: JsFieldDefinition = lindera_field.into();
        assert_eq!(back.index, 4);
        assert_eq!(back.name, "pos");
        assert!(matches!(back.field_type, JsFieldType::Custom));
    }

    #[test]
    fn test_schema_from_into_conversions() {
        let js_schema = JsSchema::new(vec![
            "surface".to_string(),
            "left_id".to_string(),
            "right_id".to_string(),
            "cost".to_string(),
        ]);

        let lindera_schema: Schema = js_schema.into();
        assert_eq!(lindera_schema.get_all_fields().len(), 4);

        let back: JsSchema = lindera_schema.into();
        assert_eq!(back.field_count(), 4);
        assert_eq!(back.get_field_name(0), Some("surface".to_string()));
    }
}

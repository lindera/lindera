use wasm_bindgen::prelude::*;

use lindera::dictionary::{FieldDefinition, FieldType, Schema};

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

impl From<FieldType> for JsFieldType {
    fn from(field_type: FieldType) -> Self {
        match field_type {
            FieldType::Surface => JsFieldType::Surface,
            FieldType::LeftContextId => JsFieldType::LeftContextId,
            FieldType::RightContextId => JsFieldType::RightContextId,
            FieldType::Cost => JsFieldType::Cost,
            FieldType::Custom => JsFieldType::Custom,
        }
    }
}

impl From<JsFieldType> for FieldType {
    fn from(field_type: JsFieldType) -> Self {
        match field_type {
            JsFieldType::Surface => FieldType::Surface,
            JsFieldType::LeftContextId => FieldType::LeftContextId,
            JsFieldType::RightContextId => FieldType::RightContextId,
            JsFieldType::Cost => FieldType::Cost,
            JsFieldType::Custom => FieldType::Custom,
        }
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

#[wasm_bindgen]
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

impl From<FieldDefinition> for JsFieldDefinition {
    fn from(field_def: FieldDefinition) -> Self {
        JsFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<JsFieldDefinition> for FieldDefinition {
    fn from(field_def: JsFieldDefinition) -> Self {
        FieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

/// Dictionary schema definition.
#[wasm_bindgen(js_name = "Schema")]
#[derive(Clone)]
pub struct JsSchema {
    pub(crate) inner: Schema,
}

#[wasm_bindgen]
impl JsSchema {
    #[wasm_bindgen(constructor)]
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            inner: Schema::new(fields),
        }
    }

    pub fn create_default() -> Self {
        Self {
            inner: Schema::default(),
        }
    }

    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.inner.get_field_index(field_name)
    }

    pub fn field_count(&self) -> usize {
        self.inner.get_all_fields().len()
    }

    pub fn get_field_name(&self, index: usize) -> Option<String> {
        self.inner.get_all_fields().get(index).cloned()
    }

    pub fn get_custom_fields(&self) -> Vec<String> {
        let fields = self.inner.get_all_fields();
        if fields.len() > 4 {
            fields[4..].to_vec()
        } else {
            Vec::new()
        }
    }

    pub fn get_all_fields(&self) -> Vec<String> {
        self.inner.get_all_fields().to_vec()
    }

    pub fn get_field_by_name(&self, name: &str) -> Option<JsFieldDefinition> {
        self.get_field_index(name).map(|index| {
            let field_type = if index < 4 {
                match index {
                    0 => JsFieldType::Surface,
                    1 => JsFieldType::LeftContextId,
                    2 => JsFieldType::RightContextId,
                    3 => JsFieldType::Cost,
                    _ => unreachable!(),
                }
            } else {
                JsFieldType::Custom
            };

            JsFieldDefinition {
                index,
                name: name.to_string(),
                field_type,
                description: None,
            }
        })
    }
}

impl From<Schema> for JsSchema {
    fn from(schema: Schema) -> Self {
        JsSchema { inner: schema }
    }
}

impl From<JsSchema> for Schema {
    fn from(schema: JsSchema) -> Self {
        schema.inner
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

        // field_count
        assert_eq!(schema.field_count(), 6);

        // get_field_index
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("pos"), Some(4));
        assert_eq!(schema.get_field_index("nonexistent"), None);

        // get_field_name
        assert_eq!(schema.get_field_name(0), Some("surface".to_string()));
        assert_eq!(schema.get_field_name(999), None);

        // get_all_fields
        let all = schema.get_all_fields();
        assert_eq!(all, fields);

        // get_custom_fields (fields beyond index 3)
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

        // Built-in field
        let surface_field = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface_field.index, 0);
        assert_eq!(surface_field.name, "surface");
        assert!(matches!(surface_field.field_type, JsFieldType::Surface));

        // Custom field
        let pos_field = schema.get_field_by_name("pos").unwrap();
        assert_eq!(pos_field.index, 4);
        assert!(matches!(pos_field.field_type, JsFieldType::Custom));

        // Non-existent field
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

        let right_id_field = schema.get_field_by_name("right_id").unwrap();
        assert_eq!(right_id_field.index, 2);
        assert!(matches!(
            right_id_field.field_type,
            JsFieldType::RightContextId
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
    fn test_field_definition_new() {
        let field = JsFieldDefinition::new(
            0,
            "surface".to_string(),
            JsFieldType::Surface,
            Some("Surface form".to_string()),
        );

        assert_eq!(field.index, 0);
        assert_eq!(field.name, "surface");
        assert!(matches!(field.field_type, JsFieldType::Surface));
        assert_eq!(field.description, Some("Surface form".to_string()));
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
        assert_eq!(
            lindera_field.description,
            Some("Part of speech".to_string())
        );

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

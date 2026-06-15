//! Shared dictionary-schema helpers for the language bindings.
//!
//! The Python/PHP/Ruby/Node.js `Schema` wrappers each reimplemented an
//! identical default field list and CSV-record validation. Those are
//! collected here so the logic lives in one place.

use lindera::dictionary::{FieldDefinition, FieldType, Schema};

use crate::error::{CoreError, CoreResult};

/// Default dictionary schema field names used by the Python/PHP/Ruby/Node.js
/// bindings' `Schema.create_default()`.
///
/// NOTE: these intentionally differ from
/// [`lindera::dictionary::Schema::default()`], which uses
/// `pos_detail_1/2/3` where these use `middle_pos/small_pos/fine_pos`. The
/// WASM binding uses `Schema::default()`, so the two field-naming schemes
/// currently diverge — a pre-existing API inconsistency left for a later
/// reconciliation. This function preserves the historical
/// Python/PHP/Ruby/Node.js naming exactly.
pub fn default_dictionary_fields() -> Vec<String> {
    [
        "surface",
        "left_context_id",
        "right_context_id",
        "cost",
        "major_pos",
        "middle_pos",
        "small_pos",
        "fine_pos",
        "conjugation_type",
        "conjugation_form",
        "base_form",
        "reading",
        "pronunciation",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

/// Validates a CSV record against the given schema field names.
///
/// Returns `Err(message)` if the record has fewer fields than the schema
/// requires, or if any schema field is present but empty. The caller maps the
/// message onto its own FFI exception type.
pub fn validate_record(fields: &[String], record: &[String]) -> Result<(), String> {
    if record.len() < fields.len() {
        return Err(format!(
            "CSV row has {} fields but schema requires {} fields",
            record.len(),
            fields.len()
        ));
    }

    for (index, field_name) in fields.iter().enumerate() {
        if index < record.len() && record[index].trim().is_empty() {
            return Err(format!("Field {field_name} is missing or empty"));
        }
    }

    Ok(())
}

/// Category of a schema field, decoupled from [`lindera::dictionary::FieldType`]
/// so the bindings depend only on `lindera-binding-core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreFieldType {
    /// Surface form (word text).
    Surface,
    /// Left context id used by the connection matrix.
    LeftContextId,
    /// Right context id used by the connection matrix.
    RightContextId,
    /// Word cost used in path selection.
    Cost,
    /// Custom field (a morphological feature).
    Custom,
}

impl CoreFieldType {
    /// Returns a stable, lowercase identifier for the field type.
    pub fn as_str(&self) -> &'static str {
        match self {
            CoreFieldType::Surface => "surface",
            CoreFieldType::LeftContextId => "left_context_id",
            CoreFieldType::RightContextId => "right_context_id",
            CoreFieldType::Cost => "cost",
            CoreFieldType::Custom => "custom",
        }
    }
}

impl From<FieldType> for CoreFieldType {
    /// Converts a lindera [`FieldType`] into a [`CoreFieldType`].
    fn from(field_type: FieldType) -> Self {
        match field_type {
            FieldType::Surface => CoreFieldType::Surface,
            FieldType::LeftContextId => CoreFieldType::LeftContextId,
            FieldType::RightContextId => CoreFieldType::RightContextId,
            FieldType::Cost => CoreFieldType::Cost,
            FieldType::Custom => CoreFieldType::Custom,
        }
    }
}

impl From<CoreFieldType> for FieldType {
    /// Converts a [`CoreFieldType`] into a lindera [`FieldType`].
    fn from(field_type: CoreFieldType) -> Self {
        match field_type {
            CoreFieldType::Surface => FieldType::Surface,
            CoreFieldType::LeftContextId => FieldType::LeftContextId,
            CoreFieldType::RightContextId => FieldType::RightContextId,
            CoreFieldType::Cost => FieldType::Cost,
            CoreFieldType::Custom => FieldType::Custom,
        }
    }
}

/// A single schema field definition, decoupled from
/// [`lindera::dictionary::FieldDefinition`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFieldDefinition {
    /// Zero-based position of the field in the schema.
    pub index: usize,
    /// Field name.
    pub name: String,
    /// Field category.
    pub field_type: CoreFieldType,
    /// Optional human-readable description.
    pub description: Option<String>,
}

impl From<FieldDefinition> for CoreFieldDefinition {
    /// Converts a lindera [`FieldDefinition`] into a [`CoreFieldDefinition`].
    fn from(field_def: FieldDefinition) -> Self {
        CoreFieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

impl From<CoreFieldDefinition> for FieldDefinition {
    /// Converts a [`CoreFieldDefinition`] into a lindera [`FieldDefinition`].
    fn from(field_def: CoreFieldDefinition) -> Self {
        FieldDefinition {
            index: field_def.index,
            name: field_def.name,
            field_type: field_def.field_type.into(),
            description: field_def.description,
        }
    }
}

/// Dictionary schema shared by the bindings.
///
/// Owns the field storage, the name-to-index map, and the field lookups by
/// delegating to [`lindera::dictionary::Schema`], so each binding can wrap a
/// `CoreSchema` instead of reimplementing the same logic.
#[derive(Debug, Clone)]
pub struct CoreSchema {
    /// The backing lindera schema that owns the fields and the index map.
    inner: Schema,
}

impl CoreSchema {
    /// Creates a schema from the given ordered field names.
    pub fn new(fields: Vec<String>) -> Self {
        Self {
            inner: Schema::new(fields),
        }
    }

    /// Creates the default binding schema (see [`default_dictionary_fields`]).
    pub fn create_default() -> Self {
        Self::new(default_dictionary_fields())
    }

    /// Returns all field names in order.
    pub fn fields(&self) -> &[String] {
        self.inner.get_all_fields()
    }

    /// Returns the index of `field_name`, if present.
    pub fn get_field_index(&self, field_name: &str) -> Option<usize> {
        self.inner.get_field_index(field_name)
    }

    /// Returns the total number of fields.
    pub fn field_count(&self) -> usize {
        self.inner.field_count()
    }

    /// Returns the field name at `index`, if present.
    pub fn get_field_name(&self, index: usize) -> Option<&str> {
        self.inner.get_field_name(index)
    }

    /// Returns the custom fields (everything after the four system fields).
    pub fn get_custom_fields(&self) -> &[String] {
        self.inner.get_custom_fields()
    }

    /// Returns the [`CoreFieldDefinition`] for `name`, if present.
    pub fn get_field_by_name(&self, name: &str) -> Option<CoreFieldDefinition> {
        self.inner
            .get_field_by_name(name)
            .map(CoreFieldDefinition::from)
    }

    /// Validates a CSV record against this schema.
    ///
    /// Returns an [`ErrorKind::Validation`](crate::ErrorKind::Validation) error
    /// when the record has too few fields or a required field is empty.
    pub fn validate_record(&self, record: &[String]) -> CoreResult<()> {
        validate_record(self.fields(), record).map_err(CoreError::validation)
    }

    /// Borrows the backing lindera schema.
    pub fn as_lindera(&self) -> &Schema {
        &self.inner
    }

    /// Consumes this schema and returns the backing lindera schema.
    pub fn into_lindera(self) -> Schema {
        self.inner
    }
}

impl From<Schema> for CoreSchema {
    /// Wraps a lindera [`Schema`] in a [`CoreSchema`].
    fn from(schema: Schema) -> Self {
        Self { inner: schema }
    }
}

impl From<CoreSchema> for Schema {
    /// Unwraps the backing lindera [`Schema`].
    fn from(schema: CoreSchema) -> Self {
        schema.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fields_count() {
        assert_eq!(default_dictionary_fields().len(), 13);
        assert_eq!(default_dictionary_fields()[0], "surface");
        assert_eq!(default_dictionary_fields()[5], "middle_pos");
    }

    #[test]
    fn validate_ok() {
        let fields = default_dictionary_fields();
        let record: Vec<String> = (0..13).map(|i| format!("v{i}")).collect();
        assert!(validate_record(&fields, &record).is_ok());
    }

    #[test]
    fn validate_too_few_fields() {
        let fields = default_dictionary_fields();
        let record = vec!["a".to_string(), "b".to_string()];
        let err = validate_record(&fields, &record).unwrap_err();
        assert!(err.contains("requires 13 fields"));
    }

    #[test]
    fn validate_empty_field() {
        let fields = vec!["surface".to_string(), "reading".to_string()];
        let record = vec!["x".to_string(), "  ".to_string()];
        let err = validate_record(&fields, &record).unwrap_err();
        assert!(err.contains("reading"));
    }

    #[test]
    fn core_field_type_roundtrips_with_lindera() {
        for ft in [
            FieldType::Surface,
            FieldType::LeftContextId,
            FieldType::RightContextId,
            FieldType::Cost,
            FieldType::Custom,
        ] {
            let core: CoreFieldType = ft.clone().into();
            let back: FieldType = core.into();
            assert_eq!(back, ft);
        }
        assert_eq!(CoreFieldType::Surface.as_str(), "surface");
        assert_eq!(CoreFieldType::Custom.as_str(), "custom");
    }

    #[test]
    fn core_field_definition_roundtrips_with_lindera() {
        let fd = FieldDefinition {
            index: 4,
            name: "major_pos".to_string(),
            field_type: FieldType::Custom,
            description: None,
        };
        let core: CoreFieldDefinition = fd.clone().into();
        assert_eq!(core.index, 4);
        assert_eq!(core.name, "major_pos");
        assert_eq!(core.field_type, CoreFieldType::Custom);
        let back: FieldDefinition = core.into();
        assert_eq!(back.index, fd.index);
        assert_eq!(back.name, fd.name);
    }

    #[test]
    fn core_schema_default_matches_binding_fields() {
        let schema = CoreSchema::create_default();
        assert_eq!(schema.field_count(), 13);
        assert_eq!(schema.fields()[0], "surface");
        // Binding default uses `middle_pos` (not lindera's `pos_detail_1`); the
        // unification is tracked separately and intentionally not done here.
        assert_eq!(schema.fields()[5], "middle_pos");
        assert_eq!(schema.fields()[12], "pronunciation");
    }

    #[test]
    fn core_schema_lookups() {
        let schema = CoreSchema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "major_pos".to_string(),
            "reading".to_string(),
        ]);
        assert_eq!(schema.get_field_index("surface"), Some(0));
        assert_eq!(schema.get_field_index("reading"), Some(5));
        assert_eq!(schema.get_field_index("nonexistent"), None);
        assert_eq!(schema.get_field_name(4), Some("major_pos"));
        assert_eq!(schema.get_field_name(99), None);
        assert_eq!(schema.get_custom_fields(), ["major_pos", "reading"]);
    }

    #[test]
    fn core_schema_get_field_by_name_classifies_type() {
        let schema = CoreSchema::create_default();
        let surface = schema.get_field_by_name("surface").unwrap();
        assert_eq!(surface.index, 0);
        assert_eq!(surface.field_type, CoreFieldType::Surface);

        let custom = schema.get_field_by_name("major_pos").unwrap();
        assert_eq!(custom.index, 4);
        assert_eq!(custom.field_type, CoreFieldType::Custom);

        assert!(schema.get_field_by_name("nonexistent").is_none());
    }

    #[test]
    fn core_schema_validate_record() {
        let schema = CoreSchema::new(vec!["surface".to_string(), "reading".to_string()]);
        assert!(
            schema
                .validate_record(&["x".to_string(), "y".to_string()])
                .is_ok()
        );
        let err = schema.validate_record(&["x".to_string()]).unwrap_err();
        assert_eq!(err.kind(), crate::ErrorKind::Validation);
    }

    #[test]
    fn core_schema_converts_to_and_from_lindera() {
        let schema = CoreSchema::new(vec!["surface".to_string(), "pos".to_string()]);
        let lindera: Schema = schema.clone().into();
        assert_eq!(lindera.get_all_fields().len(), 2);
        let back: CoreSchema = lindera.into();
        assert_eq!(back.fields(), schema.fields());
    }
}

//! Shared dictionary-schema helpers for the language bindings.
//!
//! The Python/PHP/Ruby/Node.js `Schema` wrappers each reimplemented an
//! identical default field list and CSV-record validation. Those are
//! collected here so the logic lives in one place.

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
}

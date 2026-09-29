#[cfg(feature = "embed-ipadic")]
pub mod embedded;

pub const DICTIONARY_NAME: &str = "ipadic";
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use lindera_dictionary::dictionary::metadata::Metadata;

    /// IPADIC column 8 is 活用型 (conjugation type) and column 9 is 活用形
    /// (conjugation form), as written by mecab-ipadic's `ipadic2mecabdic.pl`
    /// (`$ctype,$cform`). The schema names must follow that order (#1086).
    #[test]
    fn test_conjugation_fields_follow_ipadic_column_order() {
        let metadata = match Metadata::load(include_bytes!("../metadata.json")) {
            Ok(metadata) => metadata,
            Err(err) => panic!("failed to load metadata.json: {err}"),
        };
        let schema = &metadata.dictionary_schema;
        assert_eq!(schema.get_field_index("conjugation_type"), Some(8));
        assert_eq!(schema.get_field_index("conjugation_form"), Some(9));
    }
}

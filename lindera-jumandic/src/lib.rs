#[cfg(feature = "embed-jumandic")]
pub mod embedded;

/// Dictionary name used as the host of `embedded://` dictionary URIs.
pub const DICTIONARY_NAME: &str = "jumandic";
/// Crate version, exposed through [`get_version`].
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of this crate.
///
/// # Returns
///
/// The crate version string (e.g. `"7.1.0"`).
pub fn get_version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use lindera_dictionary::dictionary::metadata::Metadata;

    /// A line of mecab-jumandic is the surface, the two context ids and the
    /// cost, then 品詞, 品詞細分類, 活用型, 活用形, 原形, 読み and the
    /// semantic information of JUMAN: `あざまえな,1302,1302,10535,形容詞,*,
    /// ナノ形容詞,ダ列基本連体形,あざまえだ,あざまえな,自動獲得:テキスト`.
    /// The schema names follow that order.
    #[test]
    fn the_schema_follows_the_columns_of_mecab_jumandic() {
        let metadata = match Metadata::load(include_bytes!("../metadata.json")) {
            Ok(metadata) => metadata,
            Err(err) => panic!("failed to load metadata.json: {err}"),
        };
        assert_eq!(metadata.name, super::DICTIONARY_NAME);
        let schema = &metadata.dictionary_schema;
        for (index, field) in [
            "surface",
            "left_context_id",
            "right_context_id",
            "cost",
            "part_of_speech",
            "part_of_speech_subcategory_1",
            "conjugation_type",
            "conjugation_form",
            "base_form",
            "reading",
            "semantic_information",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(schema.get_field_index(field), Some(index), "{field}");
        }
    }
}

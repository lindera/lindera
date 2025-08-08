use std::error::Error;

#[cfg(feature = "embedded-unidic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        decompress::Algorithm,
        dictionary::{metadata::Metadata, schema::Schema},
        dictionary_builder::DictionaryBuilder,
    };

    let fetch_params = FetchParams {
        file_name: "unidic-mecab-2.1.2.tar.gz",
        input_dir: "unidic-mecab-2.1.2",
        output_dir: "lindera-unidic",
        dummy_input: "テスト,5131,5131,767,名詞,普通名詞,サ変可能,*,*,*,テスト,テスト-test,テスト,テスト,テスト,テスト,外,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/unidic-mecab-2.1.2.tar.gz"],
        md5_hash: "f4502a563e1da44747f61dcd2b269e35",
    };

    let metadata = Metadata::new(
        "UniDic".to_string(), // Dictionary name
        "UTF-8".to_string(),  // Encoding for UniDic
        Algorithm::Deflate,   // Compression algorithm
        3,                    // Number of fields in simple user dictionary
        -10000,               // Simple word cost
        0,                    // Simple context ID
        21,                   // Detailed user dictionary fields number
        10,                   // Unknown fields number
        true,                 // flexible_csv
        false,                // skip_invalid_cost_or_id
        false,                // normalize_details
        Schema::new(
            "UniDic".to_string(), // Schema name
            "2.1.2".to_string(),  // Schema version
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "conjugation_form".to_string(),
                "conjugation_type".to_string(),
                "lexeme_reading".to_string(),
                "lexeme".to_string(),
                "orthography_appearance".to_string(),
                "pronunciation_appearance".to_string(),
                "orthography_basic".to_string(),
                "pronunciation_basic".to_string(),
                "word_type".to_string(),
                "prefix_form".to_string(),
                "prefix_type".to_string(),
                "suffix_form".to_string(),
                "suffix_type".to_string(),
            ], // Field names
        ), // Schema for UniDic
        vec![
            Some(1), // Major POS classification
            None,    // Middle POS classification
            None,    // Small POS classification
            None,    // Fine POS classification
            None,    // Conjugation form
            None,    // Conjugation type
            Some(2), // Lexeme reading
            None,    // Lexeme
            None,    // Orthography appearance type
            None,    // Pronunciation appearance type
            None,    // Orthography basic type
            None,    // Pronunciation basic type
            None,    // Word type
            None,    // Prefix of a word form
            None,    // Prefix of a word type
            None,    // Suffix of a word form
            None,    // Suffix of a word type
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-unidic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

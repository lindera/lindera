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
        "unidic".to_string(), // Dictionary name
        "UTF-8".to_string(),  // Encoding for UniDic
        Algorithm::Deflate,   // Compression algorithm
        3,                    // Number of fields in simple user dictionary
        -10000,               // Default word cost=
        0,                    // Default left context ID
        0,                    // Default right context ID
        "*".to_string(),      // Default field value
        21,                   // Detailed user dictionary fields number
        10,                   // Unknown fields number
        true,                 // flexible_csv
        false,                // skip_invalid_cost_or_id
        false,                // normalize_details
        Schema::new(
            vec![
                "surface".to_string(),
                "left_context_id".to_string(),
                "right_context_id".to_string(),
                "cost".to_string(),
                "part_of_speech".to_string(),
                "part_of_speech_subcategory_1".to_string(),
                "part_of_speech_subcategory_2".to_string(),
                "part_of_speech_subcategory_3".to_string(),
                "conjugation_type".to_string(),
                "conjugation_form".to_string(),
                "reading".to_string(),
                "lexeme".to_string(),
                "orthographic_surface_form".to_string(),
                "phonological_surface_form".to_string(),
                "orthographic_base_form".to_string(),
                "phonological_base_form".to_string(),
                "word_type".to_string(),
                "initial_mutation_type".to_string(),
                "initial_mutation_form".to_string(),
                "final_mutation_type".to_string(),
                "final_mutation_form".to_string(),
            ], // All field names including common fields
        ), // Schema for UniDic
        vec![
            Some(1), // Part-of-speech
            None,    // Part-of-speech subcategory 1
            None,    // Part-of-speech subcategory 2
            None,    // Part-of-speech subcategory 3
            None,    // Conjugation form
            None,    // Conjugation type
            Some(2), // Reading
            None,    // Lexeme
            None,    // Orthographic surface form
            None,    // Phonological surface form
            None,    // Orthographic base form
            None,    // Phonological base form
            None,    // Word type
            None,    // Initial mutation type
            None,    // Initial mutation form
            None,    // Final mutation type
            None,    // Final mutation form
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-unidic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

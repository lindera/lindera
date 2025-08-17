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
        Schema::new(vec![
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
        ]), // Schema for UniDic dictionary
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech".to_string(),
            "reading".to_string(),
        ]), // Schema for UniDic user dictionary
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-unidic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

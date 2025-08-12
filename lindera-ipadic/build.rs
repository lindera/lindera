use std::error::Error;

#[cfg(feature = "embedded-ipadic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        decompress::Algorithm,
        dictionary::{metadata::Metadata, schema::Schema},
        dictionary_builder::DictionaryBuilder,
    };

    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-2.7.0-20070801.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20070801",
        output_dir: "lindera-ipadic",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ipadic-2.7.0-20070801.tar.gz"],
        md5_hash: "3311c7c71a869ca141e1b8bde0c8666c",
    };

    let metadata = Metadata::new(
        "ipadic".to_string(), // Dictionary name
        "EUC-JP".to_string(), // Encoding for IPADIC
        Algorithm::Deflate,   // Compression algorithm
        3,                    // Number of fields in simple user dictionary
        -10000,               // Simple word cost
        0,                    // Simple context ID
        13,                   // Detailed user dictionary fields number
        11,                   // Unknown fields number
        false,                // flexible_csv
        false,                // skip_invalid_cost_or_id
        true,                 // normalize_details
        Schema::new(
            vec![
                "major_pos".to_string(),
                "middle_pos".to_string(),
                "small_pos".to_string(),
                "fine_pos".to_string(),
                "conjugation_type".to_string(),
                "conjugation_form".to_string(),
                "base_form".to_string(),
                "reading".to_string(),
                "pronunciation".to_string(),
            ], // Field names
        ), // Schema for IPADIC
        vec![
            Some(1), // Major POS classification
            None,    // Middle POS classification
            None,    // Small POS classification
            None,    // Fine POS classification
            None,    // Conjugation type
            None,    // Conjugation form
            Some(0), // Base form
            Some(2), // Reading
            None,    // Pronunciation
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

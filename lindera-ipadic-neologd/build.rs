use std::error::Error;

#[cfg(feature = "embedded-ipadic-neologd")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        decompress::Algorithm,
        dictionary::{metadata::Metadata, schema::Schema},
        dictionary_builder::DictionaryBuilder,
    };

    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
        input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
        output_dir: "lindera-ipadic-neologd",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"],
        md5_hash: "3561f0e76980a842dc828b460a8cae96",
    };

    let metadata = Metadata::new(
        "IPADIC-NEologd".to_string(), // Dictionary name
        "UTF-8".to_string(),          // Encoding for IPADIC NEologd
        Algorithm::Deflate,           // Compression algorithm
        3,                            // Number of fields in simple user dictionary
        -10000,                       // Simple word cost
        0,                            // Simple context ID
        13,                           // Detailed user dictionary fields number
        11,                           // Unknown fields number
        false,                        // flexible_csv
        false,                        // skip_invalid_cost_or_id
        true,                         // normalize_details is true for IPAdic-NEologd
        Schema::new(
            "IPADIC-NEologd".to_string(), // Schema name
            "0.0.7-20200820".to_string(), // Schema version
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
        ), // Schema for IPADIC NEologd
        vec![
            Some(1), // Major POS classification
            None,    // Middle POS classification
            None,    // Small POS classification
            None,    // Fine POS classification
            None,    // Conjugation type
            None,    // Conjugation form
            None,    // Base form
            Some(2), // Reading
            None,    // Pronunciation
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ipadic-neologd"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

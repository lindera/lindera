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
        -10000,               // Default word cost
        0,                    // Default left context ID
        0,                    // Default right context ID
        "*".to_string(),      // Default field value
        13,                   // Detailed user dictionary fields number
        11,                   // Unknown fields number
        false,                // flexible_csv
        false,                // skip_invalid_cost_or_id
        true,                 // normalize_details
        Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "part_of_speech".to_string(),
            "part_of_speech_subcategory_1".to_string(),
            "part_of_speech_subcategory_2".to_string(),
            "part_of_speech_subcategory_3".to_string(),
            "conjugation_form".to_string(),
            "conjugation_type".to_string(),
            "base_form".to_string(),
            "reading".to_string(),
            "pronunciation".to_string(),
        ]), // Schema for IPADIC dictionary
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech".to_string(),
            "reading".to_string(),
        ]), // Schema for IPADIC user dictionary
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

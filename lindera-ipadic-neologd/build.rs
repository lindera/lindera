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
        "ipadic-neologd".to_string(), // Dictionary name
        "UTF-8".to_string(),          // Encoding for IPADIC NEologd
        Algorithm::Deflate,           // Compression algorithm
        -10000,                       // Default word cost
        0,                            // Default left context ID
        0,                            // Default right context ID
        "*".to_string(),              // Default field value
        11,                           // Unknown fields number
        false,                        // flexible_csv
        false,                        // skip_invalid_cost_or_id
        true,                         // normalize_details is true for IPAdic-NEologd
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
        ]), // Schema for IPADIC NEologd dictionary
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech".to_string(),
            "reading".to_string(),
        ]), // Schema for IPADIC-NEologd user dictionary
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ipadic-neologd"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

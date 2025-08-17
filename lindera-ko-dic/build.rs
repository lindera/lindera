use std::error::Error;

#[cfg(feature = "embedded-ko-dic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        decompress::Algorithm,
        dictionary::{metadata::Metadata, schema::Schema},
        dictionary_builder::DictionaryBuilder,
    };

    let fetch_params = FetchParams {
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        output_dir: "lindera-ko-dic",
        dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"],
        md5_hash: "b996764e91c96bc89dc32ea208514a96",
    };

    let metadata = Metadata::new(
        "ko-dic".to_string(), // Dictionary name
        "UTF-8".to_string(),  // Encoding for Ko-Dic
        Algorithm::Deflate,   // Compression algorithm
        3,                    // Number of fields in simple user dictionary
        -10000,               // Default word cost
        0,                    // Default left context ID
        0,                    // Default right context ID
        "*".to_string(),      // Default field value
        12,                   // Detailed user dictionary fields number
        12,                   // Unknown fields number
        false,                // flexible_csv
        false,                // skip_invalid_cost_or_id
        false,                // normalize_details
        Schema::new(vec![
            "surface".to_string(),
            "left_context_id".to_string(),
            "right_context_id".to_string(),
            "cost".to_string(),
            "part_of_speech_tag".to_string(),
            "meaning".to_string(),
            "presence_absence".to_string(),
            "reading".to_string(),
            "type".to_string(),
            "first_part_of_speech".to_string(),
            "last_part_of_speech".to_string(),
            "expression".to_string(),
        ]), // Schema for ko-dic dictionary
        Schema::new(vec![
            "surface".to_string(),
            "part_of_speech_tag".to_string(),
            "reading".to_string(),
        ]), // Schema for ko-dic user dictionary
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ko-dic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

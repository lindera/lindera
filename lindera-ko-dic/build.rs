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
        "KO-DIC".to_string(), // Dictionary name
        "UTF-8".to_string(),  // Encoding for Ko-Dic
        Algorithm::Deflate,   // Compression algorithm
        3,                    // Number of fields in simple user dictionary
        -10000,               // Simple word cost
        0,                    // Simple context ID
        12,                   // Detailed user dictionary fields number
        12,                   // Unknown fields number
        false,                // flexible_csv
        false,                // skip_invalid_cost_or_id
        false,                // normalize_details
        Schema::new(
            "KO-DIC".to_string(),         // Schema name
            "2.1.1-20180720".to_string(), // Schema version
            vec![
                "pos_tag".to_string(),
                "meaning".to_string(),
                "presence_absence".to_string(),
                "reading".to_string(),
                "type".to_string(),
                "first_pos".to_string(),
                "last_pos".to_string(),
                "expression".to_string(),
            ], // Field names
        ), // Schema for Ko-Dic
        vec![
            Some(1), // POS
            None,    // Meaning
            None,    // Presence or absence
            Some(2), // Reading
            None,    // Type
            None,    // First part-of-speech
            None,    // Last part-of-speech
            None,    // Expression
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ko-dic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

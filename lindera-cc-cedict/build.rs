use std::error::Error;

#[cfg(feature = "embedded-cc-cedict")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        decompress::Algorithm,
        dictionary::{metadata::Metadata, schema::Schema},
        dictionary_builder::DictionaryBuilder,
    };

    let fetch_params = FetchParams {
        file_name: "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
        input_dir: "CC-CEDICT-MeCab-0.1.0-20200409",
        output_dir: "lindera-cc-cedict",
        dummy_input: "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
        download_urls: &["https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"],
        md5_hash: "aba9748b70f37feede97b70c5d37f8a0",
    };

    let metadata = Metadata::new(
        "cc-cedict".to_string(), // Dictionary name
        "UTF-8".to_string(),     // Encoding for CC-CEDICT
        Algorithm::Deflate,      // Compression algorithm
        3,                       // Number of fields in simple user dictionary
        -10000,                  // Simple word cost
        0,                       // Simple context ID
        12,                      // Detailed user dictionary fields number
        10,                      // Unknown fields number
        true,                    // flexible_csv is true for CC-CEDICT
        true,                    // skip_invalid_cost_or_id is true for CC-CEDICT
        false,                   // normalize_details
        Schema::new(
            vec![
                "part_of_speech".to_string(),
                "part_of_speech_subcategory_1".to_string(),
                "part_of_speech_subcategory_2".to_string(),
                "part_of_speech_subcategory_3".to_string(),
                "pinyin".to_string(),
                "traditional".to_string(),
                "simplified".to_string(),
                "definition".to_string(),
            ], // Field names
        ), // Schema for CC-CEDICT
        vec![
            Some(1), // Part-of-speech
            None,    // Part-of-speech subcategory 1
            None,    // Part-of-speech subcategory 2
            None,    // Part-of-speech subcategory 3
            Some(2), // Pinyin
            None,    // Traditional
            None,    // Simplified
            None,    // Definition
        ], // User dictionary field indices
    );

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-cc-cedict"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

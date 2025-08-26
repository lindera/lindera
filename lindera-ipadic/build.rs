use std::error::Error;

#[cfg(feature = "embedded-ipadic")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use std::fs;
    use std::path::Path;

    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        dictionary::metadata::Metadata,
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

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await
}

#[cfg(not(feature = "embedded-ipadic"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

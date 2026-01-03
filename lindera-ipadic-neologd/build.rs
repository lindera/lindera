use std::error::Error;

#[cfg(feature = "embedded-ipadic-neologd")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    use std::fs;
    use std::path::Path;

    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        builder::DictionaryBuilder,
        dictionary::metadata::Metadata,
    };

    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-neologd-0.0.7-20200820.tar.gz",
        input_dir: "mecab-ipadic-neologd-0.0.7-20200820",
        output_dir: "lindera-ipadic-neologd",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ipadic-neologd-0.0.7-20200820.tar.gz"],
        md5_hash: "3561f0e76980a842dc828b460a8cae96",
    };

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await?;

    Ok(())
}

#[cfg(not(feature = "embedded-ipadic-neologd"))]
fn main() -> Result<(), Box<dyn Error>> {
    Ok(())
}

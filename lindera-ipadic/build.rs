use std::error::Error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("LINDERA_DICTIONARIES_PATH").is_none()
        && std::env::var_os("LINDERA_CACHE").is_none()
        && cfg!(not(feature = "embed-ipadic"))
    {
        return Ok(());
    }

    use std::fs;
    use std::path::Path;

    use lindera_dictionary::{
        assets::{FetchParams, fetch},
        builder::DictionaryBuilder,
        dictionary::metadata::Metadata,
    };

    let fetch_params = FetchParams {
        file_name: "mecab-ipadic-2.7.0-20250920.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20250920",
        output_dir: "lindera-ipadic",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"],
        md5_hash: "a95c409f12f1023fce8ef91f991ef042",
    };

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await?;

    Ok(())
}

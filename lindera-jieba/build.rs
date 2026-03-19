use std::error::Error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("LINDERA_DICTIONARIES_PATH").is_none()
        && std::env::var_os("LINDERA_CACHE").is_none()
        && cfg!(not(feature = "embed-jieba"))
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
        file_name: "mecab-jieba-0.1.1.tar.gz",
        input_dir: "mecab-jieba-0.1.1",
        src_subdir: Some("dict-src"),
        output_dir: "lindera-jieba",
        dummy_input: "1号店,1,1,1789,n,NUMERIC,*,*,*,*,3,1,店,low\n",
        download_urls: &["https://lindera.dev/mecab-jieba-0.1.1.tar.gz"],
        md5_hash: "749dc1ab25a035e141d014cd3c1cf8e9",
    };

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await?;

    Ok(())
}

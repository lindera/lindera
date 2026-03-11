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
        file_name: "mecab-jieba-0.1.0-20260310.tar.gz",
        input_dir: "mecab-jieba-0.1.0-20260310",
        output_dir: "lindera-jieba",
        dummy_input: "测试,0,0,-1131,*,ce4 shi4,測試,测试,to test\n",
        download_urls: &["https://lindera.dev/mecab-jieba-0.1.0-20260310.tar.gz"],
        md5_hash: "deba66bae351937d75ab895ccd5e2377",
    };

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await?;

    Ok(())
}

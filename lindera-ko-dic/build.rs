use std::error::Error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("LINDERA_DICTIONARIES_PATH").is_none()
        && std::env::var_os("LINDERA_CACHE").is_none()
        && cfg!(not(feature = "embed-ko-dic"))
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
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        output_dir: "lindera-ko-dic",
        dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &["https://Lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"],
        md5_hash: "b996764e91c96bc89dc32ea208514a96",
    };

    // Read and deserialize metadata directly from JSON file
    let metadata_path = Path::new("metadata.json");
    let metadata_json = fs::read_to_string(metadata_path)?;
    let metadata: Metadata = serde_json::from_str(&metadata_json)?;

    let builder = DictionaryBuilder::new(metadata);

    fetch(fetch_params, builder).await?;

    Ok(())
}

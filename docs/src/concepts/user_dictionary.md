# User Dictionary

A user dictionary is a supplementary dictionary that allows you to register custom words alongside the system dictionary. This is useful for domain-specific terms, brand names, proper nouns, or any words that are not in the default system dictionary.

## CSV format

The simplest user dictionary format is a CSV file with three columns:

```csv
<surface>,<part_of_speech>,<reading>
```

### Example CSV content

```csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

Each dictionary type (IPADIC, UniDic, ko-dic, etc.) also supports a detailed CSV format with full control over context IDs, costs, and all feature fields. See the [Dictionaries](./dictionaries.md) section for the detailed format of each dictionary type.

## Rust API example

```rust
use std::fs::File;
use std::path::PathBuf;

use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("user_dict")
        .join("ipadic_simple_userdic.csv");

    let metadata_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../lindera-ipadic")
        .join("metadata.json");
    let metadata: Metadata = serde_json::from_reader(
        File::open(metadata_file)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
            .unwrap(),
    )
    .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))
    .unwrap();

    let dictionary = load_dictionary("embedded://ipadic")?;
    let user_dictionary = load_user_dictionary(user_dict_path.to_str().unwrap(), &metadata)?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        Some(user_dictionary), // Using the loaded user dictionary
    );

    // Create a tokenizer.
    let tokenizer = Tokenizer::new(segmenter);

    // Tokenize a text.
    let text = "東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です";
    let mut tokens = tokenizer.tokenize(text)?;

    // Print the text and tokens.
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

Output:

```text
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

## Building a user dictionary with CLI

You can build a user dictionary from CSV to binary format using the CLI:

```shell
lindera build --src <source_dir> --dest <dest_dir> --metadata <metadata.json> --user
```

### Binary vs CSV user dictionary

- **CSV format**: Loaded and parsed at runtime. Convenient for development and small dictionaries.
- **Binary format**: Pre-compiled for faster loading. Recommended for production use with large user dictionaries.

Both formats can be specified when creating a `Segmenter`. The binary format skips the CSV parsing step, resulting in faster startup times.

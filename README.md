# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

A morphological analysis library in Rust. This project is forked from [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

Lindera aims to build a library which is easy to install and provides concise APIs for various Rust applications.

## Tokenization examples

### Basic tokenization

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.2.0", features = ["embedded-ipadic"] }
```

This example covers the basic usage of Lindera.

It will:

- Create a tokenizer in normal mode
- Tokenize the input text
- Output the tokens

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = tokenizer.tokenize(text)?;
    println!("text:\t{}", text);
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

The above example can be run as follows:

```shell
% cargo run --features=embedded-ipadic --example=tokenize
```

You can see the result as follows:

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    UNK
```

### Tokenization with user dictionary

You can give user dictionary entries along with the default system dictionary. User dictionary should be a CSV with following format.

```csv
<surface>,<part_of_speech>,<reading>
```

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.2.0", features = ["embedded-ipadic"] }
```

For example:

```shell
% cat ./resources/user_dict/ipadic_simple_userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

With an user dictionary, `Tokenizer` will be created as follows:

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

The above example can be run by `cargo run --example`:

```shell
% cargo run --features=embedded-ipadic --example=tokenize_with_user_dict
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

### Tokenize with filters

Put the following in Cargo.toml:

```toml
[dependencies]
lindera = { version = "1.2.0", features = ["embedded-ipadic"] }
```

This example covers the basic usage of Lindera Analysis Framework.

It will:

- Apply character filter for Unicode normalization (NFKC)
- Tokenize the input text with IPADIC
- Apply token filters for removing stop tags (Part-of-speech) and Japanese Katakana stem filter

```rust
use lindera::character_filter::BoxCharacterFilter;
use lindera::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
use lindera::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::token_filter::BoxTokenFilter;
use lindera::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
use lindera::token_filter::japanese_number::JapaneseNumberTokenFilter;
use lindera::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(
        Mode::Normal,
        dictionary,
        None, // No user dictionary for this example
    );

    let unicode_normalize_char_filter =
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

    let japanese_iteration_mark_char_filter =
        JapaneseIterationMarkCharacterFilter::new(true, true);

    let japanese_compound_word_token_filter = JapaneseCompoundWordTokenFilter::new(
        vec!["名詞,数".to_string(), "名詞,接尾,助数詞".to_string()]
            .into_iter()
            .collect(),
        Some("複合語".to_string()),
    );

    let japanese_number_token_filter =
        JapaneseNumberTokenFilter::new(Some(vec!["名詞,数".to_string()].into_iter().collect()));

    let japanese_stop_tags_token_filter = JapaneseStopTagsTokenFilter::new(
        vec![
            "接続詞".to_string(),
            "助詞".to_string(),
            "助詞,格助詞".to_string(),
            "助詞,格助詞,一般".to_string(),
            "助詞,格助詞,引用".to_string(),
            "助詞,格助詞,連語".to_string(),
            "助詞,係助詞".to_string(),
            "助詞,副助詞".to_string(),
            "助詞,間投助詞".to_string(),
            "助詞,並立助詞".to_string(),
            "助詞,終助詞".to_string(),
            "助詞,副助詞／並立助詞／終助詞".to_string(),
            "助詞,連体化".to_string(),
            "助詞,副詞化".to_string(),
            "助詞,特殊".to_string(),
            "助動詞".to_string(),
            "記号".to_string(),
            "記号,一般".to_string(),
            "記号,読点".to_string(),
            "記号,句点".to_string(),
            "記号,空白".to_string(),
            "記号,括弧閉".to_string(),
            "その他,間投".to_string(),
            "フィラー".to_string(),
            "非言語音".to_string(),
        ]
        .into_iter()
        .collect(),
    );

    // Create a tokenizer.
    let mut tokenizer = Tokenizer::new(segmenter);

    tokenizer
        .append_character_filter(BoxCharacterFilter::from(unicode_normalize_char_filter))
        .append_character_filter(BoxCharacterFilter::from(
            japanese_iteration_mark_char_filter,
        ))
        .append_token_filter(BoxTokenFilter::from(japanese_compound_word_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_number_token_filter))
        .append_token_filter(BoxTokenFilter::from(japanese_stop_tags_token_filter));

    // Tokenize a text.
    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。";
    let tokens = tokenizer.tokenize(text)?;

    // Print the text and tokens.
    println!("text: {}", text);
    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.surface, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

The above example can be run as follows:

```shell
% cargo run --features=embedded-ipadic --example=tokenize_with_filters
```

You can see the result as follows:

```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: "Lindera", start: 0, end: 21, details: Some(["UNK"])
token: "形態素", start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "ユーザー", start: 63, end: 75, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: "辞書", start: 75, end: 81, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: "利用", start: 84, end: 90, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: "可能", start: 90, end: 96, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```

## Configuration file

Lindera is able to read YAML format configuration files.
Specify the path to the following file in the environment variable LINDERA_CONFIG_PATH. You can use it easily without having to code the behavior of the tokenizer in Rust code.

```yaml
segmenter:
  mode: "normal"
  dictionary:
    kind: "ipadic"
  user_dictionary:
    path: "./resources/user_dict/ipadic_simple.csv"
    kind: "ipadic"

character_filters:
  - kind: "unicode_normalize"
    args:
      kind: "nfkc"
  - kind: "japanese_iteration_mark"
    args:
      normalize_kanji: true
      normalize_kana: true
  - kind: mapping
    args:
       mapping:
         リンデラ: Lindera

token_filters:
  - kind: "japanese_compound_word"
    args:
      tags:
        - "名詞,数"
        - "名詞,接尾,助数詞"
      new_tag: "名詞,数"
  - kind: "japanese_number"
    args:
      tags:
        - "名詞,数"
  - kind: "japanese_stop_tags"
    args:
      tags:
        - "接続詞"
        - "助詞"
        - "助詞,格助詞"
        - "助詞,格助詞,一般"
        - "助詞,格助詞,引用"
        - "助詞,格助詞,連語"
        - "助詞,係助詞"
        - "助詞,副助詞"
        - "助詞,間投助詞"
        - "助詞,並立助詞"
        - "助詞,終助詞"
        - "助詞,副助詞／並立助詞／終助詞"
        - "助詞,連体化"
        - "助詞,副詞化"
        - "助詞,特殊"
        - "助動詞"
        - "記号"
        - "記号,一般"
        - "記号,読点"
        - "記号,句点"
        - "記号,空白"
        - "記号,括弧閉"
        - "その他,間投"
        - "フィラー"
        - "非言語音"
  - kind: "japanese_katakana_stem"
    args:
      min: 3
  - kind: "remove_diacritical_mark"
    args:
      japanese: false
```

```shell
% export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera::tokenizer::TokenizerBuilder;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // Load tokenizer configuration from file
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
        .join("config")
        .join("lindera.yml");

    let builder = TokenizerBuilder::from_file(&path)?;

    let tokenizer = builder.build()?;

    let text = "Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。".to_string();
    println!("text: {text}");

    let tokens = tokenizer.tokenize(&text)?;

    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.surface, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

## Environment Variables

### LINDERA_CACHE

The `LINDERA_CACHE` environment variable specifies a directory for caching dictionary source files. This enables:

- **Offline builds**: Once downloaded, dictionary source files are preserved for future builds
- **Faster builds**: Subsequent builds skip downloading if valid cached files exist
- **Reproducible builds**: Ensures consistent dictionary versions across builds

Usage:

```shell
export LINDERA_CACHE=/path/to/cache
cargo build --features=ipadic
```

When set, dictionary source files are stored in `$LINDERA_CACHE/<version>/` where `<version>` is the lindera-dictionary crate version. The cache validates files using MD5 checksums - invalid files are automatically re-downloaded.

### LINDERA_CONFIG_PATH

The `LINDERA_CONFIG_PATH` environment variable specifies the path to a YAML configuration file for the tokenizer. This allows you to configure tokenizer behavior without modifying Rust code.

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

See the [Configuration file](#configuration-file) section for details on the configuration format.

### DOCS_RS

The `DOCS_RS` environment variable is automatically set by docs.rs when building documentation. When this variable is detected, Lindera creates dummy dictionary files instead of downloading actual dictionary data, allowing documentation to be built without network access or large file downloads.

This is primarily used internally by docs.rs and typically doesn't need to be set by users.

### LINDERA_WORKDIR

The `LINDERA_WORKDIR` environment variable is automatically set during the build process by the lindera-dictionary crate. It points to the directory containing the built dictionary data files and is used internally by dictionary crates to locate their data files.

This variable is set automatically and should not be modified by users.

## Dictionary Training (Experimental)

Lindera provides CRF-based dictionary training functionality for creating custom morphological analysis models.

### Overview

Lindera Trainer is a Conditional Random Field (CRF) based morphological analyzer training system with the following advanced features:

- **CRF-based statistical learning**: Efficient implementation using rucrf crate
- **L1 regularization**: Prevents overfitting
- **Multi-threaded training**: Parallel processing for faster training
- **Comprehensive Unicode support**: Full CJK extension support
- **Advanced unknown word handling**: Intelligent mixed character type classification
- **Multi-stage weight optimization**: Advanced normalization system for trained weights
- **Lindera dictionary compatibility**: Full compatibility with existing dictionary formats

### CLI Usage

For detailed CLI command usage, see [lindera-cli/README.md](./lindera-cli/README.md#train-dictionary).

### Required File Format Specifications

#### 1. **Vocabulary Dictionary (seed.csv)**

**Role**: Base vocabulary dictionary
**Format**: MeCab format CSV

```csv
外国,0,0,0,名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人,0,0,0,名詞,接尾,一般,*,*,*,人,ジン,ジン
参政,0,0,0,名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
```

- **Purpose**: Define basic words and their part-of-speech information for training
- **Structure**: `surface,left_id,right_id,cost,pos,pos_detail1,pos_detail2,pos_detail3,inflection_type,inflection_form,base_form,reading,pronunciation`

#### 2. **Unknown Word Definition (unk.def)**

**Role**: Unknown word processing definition
**Format**: Unknown word parameters by character type

```csv
DEFAULT,0,0,0,名詞,一般,*,*,*,*,*,*,*
HIRAGANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KATAKANA,0,0,0,名詞,一般,*,*,*,*,*,*,*
KANJI,0,0,0,名詞,一般,*,*,*,*,*,*,*
ALPHA,0,0,0,名詞,固有名詞,一般,*,*,*,*,*,*
NUMERIC,0,0,0,名詞,数,*,*,*,*,*,*,*
```

- **Purpose**: Define processing methods for out-of-vocabulary words by character type
- **Note**: These labels are for internal processing and are not output in the final dictionary file

#### 3. **Training Corpus (corpus.txt)**

**Role**: Training data (annotated corpus)
**Format**: Tab-separated tokenized text

```text
外国	名詞,一般,*,*,*,*,外国,ガイコク,ガイコク
人	名詞,接尾,一般,*,*,*,人,ジン,ジン
参政	名詞,サ変接続,*,*,*,*,参政,サンセイ,サンセイ
権	名詞,接尾,一般,*,*,*,権,ケン,ケン
EOS

これ	連体詞,*,*,*,*,*,これ,コレ,コレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
テスト	名詞,サ変接続,*,*,*,*,テスト,テスト,テスト
EOS
```

- **Purpose**: Sentences and their correct analysis results for training
- **Format**: Each line is `surface\tpos_info`, sentences end with `EOS`
- **Important**: Training quality heavily depends on the quantity and quality of this corpus

#### 4. **Character Type Definition (char.def)**

**Role**: Character type definition
**Format**: Character categories and character code ranges

```text
# Character category definition (category_name compatibility_flag continuity_flag length)
DEFAULT 0 1 0
HIRAGANA 1 1 0
KATAKANA 1 1 0
KANJI 0 0 2
ALPHA 1 1 0
NUMERIC 1 1 0

# Character range mapping
0x3041..0x3096 HIRAGANA  # Hiragana
0x30A1..0x30F6 KATAKANA  # Katakana
0x4E00..0x9FAF KANJI     # Kanji
0x0030..0x0039 NUMERIC   # Numbers
0x0041..0x005A ALPHA     # Uppercase letters
0x0061..0x007A ALPHA     # Lowercase letters
```

- **Purpose**: Define which characters belong to which category
- **Parameters**: Settings for compatibility, continuity, default length, etc.

#### 5. **Feature Template (feature.def)**

**Role**: Feature template definition
**Format**: Feature extraction patterns

```text
# Unigram features (word-level features)
UNIGRAM:%F[0]         # POS (feature element 0)
UNIGRAM:%F[1]         # POS detail 1
UNIGRAM:%F[6]         # Base form
UNIGRAM:%F[7]         # Reading (Katakana)

# Left context features
LEFT:%L[0]            # POS of left word
LEFT:%L[1]            # POS detail of left word

# Right context features
RIGHT:%R[0]           # POS of right word
RIGHT:%R[1]           # POS detail of right word

# Bigram features (combination features)
UNIGRAM:%F[0]/%F[1]   # POS + POS detail
UNIGRAM:%F[0]/%F[6]   # POS + base form
```

- **Purpose**: Define which information to extract features from
- **Templates**: `%F[n]` (feature), `%L[n]` (left context), `%R[n]` (right context)

#### 6. **Feature Normalization Rules (rewrite.def)**

**Role**: Feature normalization rules
**Format**: Replacement rules (tab-separated)

```text
# Normalize numeric expressions
数	NUM
*	UNK

# Normalize proper nouns
名詞,固有名詞	名詞,一般

# Simplify auxiliary verbs
助動詞,*,*,*,特殊・デス	助動詞
助動詞,*,*,*,特殊・ダ	助動詞
```

- **Purpose**: Normalize features to improve training efficiency
- **Format**: `original_pattern\treplacement_pattern`
- **Effect**: Generalize rare features to reduce sparsity problems

#### 7. **Output Model Format**

**Role**: Output model file
**Format**: Binary (rkyv) format is standard, JSON format also supported

The model contains the following information:

```json
{
  "feature_weights": [0.0, 0.084, 0.091, ...],
  "labels": ["外国", "人", "参政", "権", ...],
  "pos_info": ["名詞,一般,*,*,*,*,*,*,*", "名詞,接尾,一般,*,*,*,*,*,*", ...],
  "feature_templates": ["UNIGRAM:%F[0]", ...],
  "metadata": {
    "version": "1.0.0",
    "regularization": 0.01,
    "iterations": 100,
    "feature_count": 13,
    "label_count": 19
  }
}
```

- **Purpose**: Save training results for later dictionary generation

### Training Parameter Specifications

- **Regularization coefficient (lambda)**: Controls L1 regularization strength (default: 0.01)
- **Maximum iterations (iter)**: Maximum number of training iterations (default: 100)
- **Parallel threads (threads)**: Number of parallel processing threads (default: 1)

### API Usage Example

```rust,no_run
use std::fs::File;
use lindera_dictionary::trainer::{Corpus, Trainer, TrainerConfig};

// Load configuration from files
let seed_file = File::open("resources/training/seed.csv")?;
let char_file = File::open("resources/training/char.def")?;
let unk_file = File::open("resources/training/unk.def")?;
let feature_file = File::open("resources/training/feature.def")?;
let rewrite_file = File::open("resources/training/rewrite.def")?;

let config = TrainerConfig::from_readers(
    seed_file,
    char_file,
    unk_file,
    feature_file,
    rewrite_file
)?;

// Initialize and configure trainer
let trainer = Trainer::new(config)?
    .regularization_cost(0.01)
    .max_iter(100)
    .num_threads(4);

// Load corpus
let corpus_file = File::open("resources/training/corpus.txt")?;
let corpus = Corpus::from_reader(corpus_file)?;

// Execute training
let model = trainer.train(corpus)?;

// Save model (binary format)
let mut output = File::create("trained_model.dat")?;
model.write_model(&mut output)?;

// Output in Lindera dictionary format
let mut lex_out = File::create("output_lex.csv")?;
let mut conn_out = File::create("output_conn.dat")?;
let mut unk_out = File::create("output_unk.def")?;
let mut user_out = File::create("output_user.csv")?;
model.write_dictionary(&mut lex_out, &mut conn_out, &mut unk_out, &mut user_out)?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

### Implementation Status

#### Completed Features

##### **Core Features**

- **Core architecture**: Complete trainer module structure
- **CRF training**: Conditional Random Field training via rucrf integration
- **CLI integration**: `lindera train` command with full parameter support
- **Corpus processing**: Full MeCab format corpus support
- **Dictionary integration**: Dictionary construction from seed.csv, char.def, unk.def
- **Feature extraction**: Extraction and transformation of unigram/bigram features
- **Model saving**: Output trained models in JSON/rkyv format
- **Dictionary output**: Generate Lindera format dictionary files

##### **Advanced Unknown Word Processing**

- **Comprehensive Unicode support**: Full support for CJK extensions, Katakana extensions, Hiragana extensions
- **Category-specific POS assignment**: Automatic assignment of appropriate POS information by character type
  - DEFAULT: 名詞,一般 (unknown character type)
  - HIRAGANA/KATAKANA/KANJI: 名詞,一般 (Japanese characters)
  - ALPHA: 名詞,固有名詞 (alphabetic characters)
  - NUMERIC: 名詞,数 (numeric characters)
- **Surface form analysis**: Feature generation based on character patterns, length, and position information
- **Dynamic cost calculation**: Adaptive cost considering character type and context

##### **Refactored Implementation (September 2024 Latest)**

- **Constant management**: Magic number elimination via cost_constants module
- **Method splitting**: Improved readability by splitting large methods
  - `train()` → `build_lattices_from_corpus()`, `extract_labels()`, `train_crf_model()`, `create_final_model()`
- **Unified cost calculation**: Improved maintainability by unifying duplicate code
  - `calculate_known_word_cost()`: Known word cost calculation
  - `calculate_unknown_word_cost()`: Unknown word cost calculation
- **Organized debug output**: Structured logging via log_debug! macro
- **Enhanced error handling**: Comprehensive error handling and documentation

### Architecture

```text
lindera-dictionary/src/trainer.rs  # Main Trainer struct
lindera-dictionary/src/trainer/
├── config.rs           # Configuration management
├── corpus.rs           # Corpus processing
├── feature_extractor.rs # Feature extraction
├── feature_rewriter.rs  # Feature rewriting
└── model.rs            # Trained model
```

### Advanced Unknown Word Processing System

#### Comprehensive Unicode Character Type Detection

The latest implementation significantly extends the basic Unicode ranges and fully supports the following character sets. (See the Category-specific POS assignment details in the Advanced Unknown Word Processing section above.)

#### Feature Weight Optimization

##### **Cost Calculation Constants**

```rust
mod cost_constants {
    // Known word cost calculation
    pub const KNOWN_WORD_BASE_COST: i16 = 1000;
    pub const KNOWN_WORD_COST_MULTIPLIER: f64 = 500.0;
    pub const KNOWN_WORD_COST_MIN: i16 = 500;
    pub const KNOWN_WORD_COST_MAX: i16 = 3000;
    pub const KNOWN_WORD_DEFAULT_COST: i16 = 1500;

    // Unknown word cost calculation
    pub const UNK_BASE_COST: i32 = 3000;
    pub const UNK_COST_MULTIPLIER: f64 = 500.0;
    pub const UNK_COST_MIN: i32 = 2500;
    pub const UNK_COST_MAX: i32 = 4500;

    // Category-specific adjustments
    pub const UNK_DEFAULT_ADJUSTMENT: i32 = 0;     // DEFAULT
    pub const UNK_HIRAGANA_ADJUSTMENT: i32 = 200;  // HIRAGANA - minor penalty
    pub const UNK_KATAKANA_ADJUSTMENT: i32 = 0;    // KATAKANA - medium
    pub const UNK_KANJI_ADJUSTMENT: i32 = 400;     // KANJI - high penalty
    pub const UNK_ALPHA_ADJUSTMENT: i32 = 100;     // ALPHA - mild penalty
    pub const UNK_NUMERIC_ADJUSTMENT: i32 = -100;  // NUMERIC - bonus (regular)
}
```

##### **Unified Cost Calculation**

```rust
// Known word cost calculation
fn calculate_known_word_cost(&self, feature_weight: f64) -> i16 {
    let scaled_weight = (feature_weight * cost_constants::KNOWN_WORD_COST_MULTIPLIER) as i32;
    let final_cost = cost_constants::KNOWN_WORD_BASE_COST as i32 + scaled_weight;
    final_cost.clamp(
        cost_constants::KNOWN_WORD_COST_MIN as i32,
        cost_constants::KNOWN_WORD_COST_MAX as i32
    ) as i16
}

// Unknown word cost calculation
fn calculate_unknown_word_cost(&self, feature_weight: f64, category: usize) -> i32 {
    let base_cost = cost_constants::UNK_BASE_COST;
    let category_adjustment = match category {
        0 => cost_constants::UNK_DEFAULT_ADJUSTMENT,
        1 => cost_constants::UNK_HIRAGANA_ADJUSTMENT,
        2 => cost_constants::UNK_KATAKANA_ADJUSTMENT,
        3 => cost_constants::UNK_KANJI_ADJUSTMENT,
        4 => cost_constants::UNK_ALPHA_ADJUSTMENT,
        5 => cost_constants::UNK_NUMERIC_ADJUSTMENT,
        _ => 0,
    };
    let scaled_weight = (feature_weight * cost_constants::UNK_COST_MULTIPLIER) as i32;
    let final_cost = base_cost + category_adjustment + scaled_weight;
    final_cost.clamp(
        cost_constants::UNK_COST_MIN,
        cost_constants::UNK_COST_MAX
    )
}
```

### Performance Optimization

#### Memory Efficiency

- **Lazy evaluation**: Create merged_model only when needed
- **Unused feature removal**: Automatic deletion of unnecessary features after training
- **Efficient binary format**: Fast serialization using rkyv

#### Parallel Processing Support

```rust
let trainer = rucrf::Trainer::new()
    .regularization(rucrf::Regularization::L1, regularization_cost)?
    .max_iter(max_iter)?
    .n_threads(self.num_threads)?;  // Multi-threaded training
```

### Practical Training Data Requirements

#### Recommended Corpus Specifications

Recommendations for generating effective dictionaries for real applications:

1. **Corpus Size**
   - **Minimum**: 100 sentences (for basic operation verification)
   - **Recommended**: 1,000+ sentences (practical level)
   - **Ideal**: 10,000+ sentences (commercial quality)

2. **Vocabulary Diversity**
   - Balanced distribution of different parts of speech
   - Coverage of inflections and suffixes
   - Appropriate inclusion of technical terms and proper nouns

3. **Quality Control**
   - Manual verification of morphological analysis results
   - Consistent application of analysis criteria
   - Maintain error rate below 5%

## API reference

The API reference is available. Please see following URL:

- [lindera](https://docs.rs/lindera)

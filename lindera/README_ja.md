# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

Rust による形態素解析ライブラリ。このプロジェクトは [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs) からフォークされたものです。

Lindera は、さまざまな Rust アプリケーション向けに、簡単にインストールでき、簡潔な API を提供するライブラリの構築を目指しています。

## トークナイズの使用例

### 基本的なトークナイズ

Cargo.toml に以下を記述してください:

```toml
[dependencies]
lindera = { version = "3.0.0", features = ["embed-ipadic"] }
```

この例では Lindera の基本的な使い方を説明します。

以下の処理を行います:

- Normal モードでトークナイザを作成
- 入力テキストをトークナイズ
- トークンを出力

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

上記の例は以下のように実行できます:

```shell
% cargo run --features=embed-ipadic --example=tokenize
```

実行結果は以下の通りです:

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    UNK
```

### ユーザー辞書を使ったトークナイズ

デフォルトのシステム辞書に加えて、ユーザー辞書エントリを指定できます。ユーザー辞書は以下のフォーマットの CSV ファイルです。

```csv
<surface>,<part_of_speech>,<reading>
```

Cargo.toml に以下を記述してください:

```toml
[dependencies]
lindera = { version = "3.0.0", features = ["embed-ipadic"] }
```

例:

```shell
% cat ./resources/simple_userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

ユーザー辞書を使用する場合、`Tokenizer` は以下のように作成します:

```rust
use std::path::PathBuf;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;

fn main() -> LinderaResult<()> {
    let user_dict_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
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
        Some(user_dictionary), // Assuming no user dictionary is provided
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

上記の例は `cargo run --example` で実行できます:

```shell
% cargo run --features=embed-ipadic --example=tokenize_with_user_dict
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

### フィルタを使ったトークナイズ

Cargo.toml に以下を記述してください:

```toml
[dependencies]
lindera = { version = "3.0.0", features = ["embed-ipadic"] }
```

この例では Lindera Analysis Framework の基本的な使い方を説明します。

以下の処理を行います:

- Unicode 正規化（NFKC）用の文字フィルタを適用
- IPADIC で入力テキストをトークナイズ
- ストップタグ（品詞）の除去と日本語カタカナ語幹フィルタのトークンフィルタを適用

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
        None, // Assuming no user dictionary is provided
    );

    let unicode_normalize_char_filter =
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC);

    let japanese_iterration_mark_char_filter =
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
            japanese_iterration_mark_char_filter,
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

上記の例は以下のように実行できます:

```shell
% cargo run --features=embed-ipadic --example=tokenize_with_filters
```

実行結果は以下の通りです:

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

## 設定ファイル

Lindera は YAML 形式の設定ファイルを読み込むことができます。
以下のファイルへのパスを環境変数 LINDERA_CONFIG_PATH に指定してください。Rust コードでトークナイザの動作をコーディングすることなく、簡単に利用できます。

```yaml
segmenter:
  mode: "normal"
  dictionary: "embedded://ipadic"
  # user_dictionary: "./resources/user_dict/ipadic_simple_userdic.csv"

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
      kind: "ipadic"
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
% export LINDERA_CONFIG_PATH=./resources/lindera.yml
```

```rust
use std::path::PathBuf;

use lindera::tokenizer::TokenizerBuilder;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    // Creates a new `TokenizerConfigBuilder` instance.
    // If the `LINDERA_CONFIG_PATH` environment variable is set, it will attempt to load the initial settings from the specified path.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../resources")
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

## 環境変数

### LINDERA_DICTIONARIES_PATH

`LINDERA_DICTIONARIES_PATH` 環境変数は、辞書ソースファイルのキャッシュディレクトリを指定します。以下の機能を提供します:

- **オフラインビルド**: 一度ダウンロードした辞書ソースファイルは、以降のビルドで再利用されます
- **高速ビルド**: 有効なキャッシュファイルが存在する場合、以降のビルドでダウンロードをスキップします
- **再現性のあるビルド**: ビルド間で一貫した辞書バージョンを保証します

使用方法:

```shell
export LINDERA_DICTIONARIES_PATH=/path/to/cache
cargo build --features=ipadic
```

設定すると、辞書ソースファイルは `$LINDERA_DICTIONARIES_PATH/<version>/` に保存されます。ここで `<version>` は lindera-dictionary クレートのバージョンです。キャッシュは MD5 チェックサムでファイルを検証し、無効なファイルは自動的に再ダウンロードされます。

### LINDERA_CONFIG_PATH

`LINDERA_CONFIG_PATH` 環境変数は、トークナイザの YAML 設定ファイルへのパスを指定します。Rust コードを変更することなく、トークナイザの動作を設定できます。

```shell
export LINDERA_CONFIG_PATH=./resources/lindera.yml
```

設定ファイルのフォーマットの詳細は[設定ファイル](#設定ファイル)セクションを参照してください。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rs がドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Lindera は実際の辞書データをダウンロードする代わりにダミー辞書ファイルを作成し、ネットワークアクセスや大容量ファイルのダウンロードなしにドキュメントをビルドできるようにします。

これは主に docs.rs で内部的に使用されるもので、通常ユーザーが設定する必要はありません。

### LINDERA_WORKDIR

`LINDERA_WORKDIR` 環境変数は、lindera-dictionary クレートによってビルドプロセス中に自動的に設定されます。ビルドされた辞書データファイルを含むディレクトリを指し、辞書クレートがデータファイルの場所を特定するために内部的に使用されます。

この変数は自動的に設定されるため、ユーザーが変更する必要はありません。

## API リファレンス

API リファレンスは以下の URL を参照してください:

- [lindera](https://docs.rs/lindera)

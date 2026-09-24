# Lindera

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Crates.io](https://img.shields.io/crates/v/lindera.svg)](https://crates.io/crates/lindera)

Rust による形態素解析ライブラリです。このプロジェクトは [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs) からフォークしたものです。

Lindera は、インストールが簡単で、さまざまな Rust アプリケーションに簡潔な API を提供するライブラリを目指しています。

## Feature フラグ

v5.0 以降、このクレートは `Segmenter` API を中心とした純粋な形態素分割器（セグメンター）です。分析チェーン（文字フィルタ、トークンフィルタ、`Tokenizer`）は、姉妹クレートの [`lindera-analysis`](https://crates.io/crates/lindera-analysis) が提供します。

```toml
[dependencies]
# Pure segmenter
lindera = "6"

# With the analysis chain (character filters, token filters, Tokenizer)
lindera = "6"
lindera-analysis = "6"
```

このクレートの主な feature フラグは次のとおりです。

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| `mmap` | メモリマップによる辞書の読み込み | 有効 |
| `train` | CRF ベースの辞書学習（`lindera-trainer` に依存） | 無効 |
| `embed-ipadic`、`embed-ipadic-neologd`、`embed-unidic`、`embed-sudachidict` | 日本語辞書をバイナリに埋め込む | 無効 |
| `embed-ko-dic` | 韓国語辞書（ko-dic）をバイナリに埋め込む | 無効 |
| `embed-cc-cedict`、`embed-jieba` | 中国語辞書をバイナリに埋め込む | 無効 |

以下の例では `embed-ipadic` を使います。`embed-*` feature を使わない場合は、ビルド済みの辞書をパスから読み込みます（`load_dictionary("/path/to/ipadic")`）。`embed-cjk*` のような組み合わせを含む一覧は、[Feature フラグ](https://lindera.github.io/lindera/ja/development/feature_flags.html)を参照してください。

v5 からアップグレードする場合は、[移行ガイド](https://lindera.github.io/lindera/ja/migration_v5_to_v6.html)を参照してください。

## セグメンテーションの例

### 基本的なセグメンテーション

Cargo.toml に以下を追加します。

```toml
[dependencies]
lindera = { version = "6", features = ["embed-ipadic"] }
```

この例では、追加のクレートを使わずに、純粋なセグメンターとしての Lindera の基本的な使い方を説明します。

以下の処理を行います。

- Normal モードでセグメンターを作成
- 入力テキストを分割
- トークンを出力

```rust
use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = segmenter.segment(Cow::Borrowed(text))?;
    println!("text:\t{text}");
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

上記の例は以下のように実行できます。

```shell
% cargo run -p lindera --features=embed-ipadic --example=segment
```

実行結果は以下のとおりです。

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## トークナイズの例

以下の `Tokenizer` とフィルタチェーンは、`lindera-analysis` クレートが提供します。

```toml
[dependencies]
lindera = { version = "6", features = ["embed-ipadic"] }
lindera-analysis = "6"
```

### 基本的なトークナイズ

この例では、トークナイザーの基本的な使い方を説明します。

以下の処理を行います。

- Normal モードでトークナイザーを作成
- 入力テキストをトークナイズ
- トークンを出力

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "関西国際空港限定トートバッグ";
    let mut tokens = tokenizer.tokenize(text)?;
    println!("text:\t{text}");
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

上記の例は以下のように実行できます。

```shell
% cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize
```

実行結果は以下のとおりです。

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

### ユーザー辞書を使ったトークナイズ

システム辞書に加えて、ユーザー辞書のエントリを指定できます。ユーザー辞書は以下の形式の CSV です。

```csv
<surface>,<part_of_speech>,<reading>
```

Cargo.toml に以下を追加します。

```toml
[dependencies]
lindera = { version = "6", features = ["embed-ipadic"] }
lindera-analysis = "6"
anyhow = "1"
serde_json = "1"
```

たとえば次のような内容です。

```shell
% cat ./resources/user_dict/ipadic_simple_userdic.csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

ユーザー辞書を使う場合、`Tokenizer` は次のように作成します。

```rust
use std::fs::File;
use std::path::PathBuf;

use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::Tokenizer;
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
    println!("text:\t{text}");
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("token:\t{}\t{}", token.surface.as_ref(), details);
    }

    Ok(())
}
```

上記の例は `cargo run --example` で実行できます。

```shell
% cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_user_dict
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,*,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,*,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

### フィルタを使ったトークナイズ

Cargo.toml に以下を追加します。

```toml
[dependencies]
lindera = { version = "6", features = ["embed-ipadic"] }
lindera-analysis = "6"
```

この例では、文字フィルタとトークンフィルタで分析チェーンを組み立てます。

以下の処理を行います。

- Unicode 正規化（NFKC）と日本語の踊り字（繰り返し記号）の文字フィルタを適用
- IPADIC で入力テキストをトークナイズ
- 連続する数詞トークンの結合、漢数字のアラビア数字への変換、品詞タグによるトークンの除去を行うトークンフィルタを適用

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::character_filter::BoxCharacterFilter;
use lindera_analysis::character_filter::japanese_iteration_mark::JapaneseIterationMarkCharacterFilter;
use lindera_analysis::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera_analysis::token_filter::BoxTokenFilter;
use lindera_analysis::token_filter::japanese_compound_word::JapaneseCompoundWordTokenFilter;
use lindera_analysis::token_filter::japanese_number::JapaneseNumberTokenFilter;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera_analysis::tokenizer::Tokenizer;
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

    // Merge the numeral tokens only. A counter merged into the number token
    // would keep the number filter below from converting it.
    let japanese_compound_word_token_filter = JapaneseCompoundWordTokenFilter::new(
        vec!["名詞,数".to_string()].into_iter().collect(),
        Some("名詞,数".to_string()),
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
    println!("text: {text}");
    for token in tokens {
        println!(
            "token: {:?}, start: {:?}, end: {:?}, details: {:?}",
            token.surface, token.byte_start, token.byte_end, token.details
        );
    }

    Ok(())
}
```

上記の例は以下のように実行できます。

```shell
% cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_filters
```

実行結果は以下のとおりです。

```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: "Lindera", start: 0, end: 21, details: Some(["名詞", "固有名詞", "組織", "*", "*", "*", "*", "*", "*"])
token: "形態素", start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "ユーザー", start: 63, end: 75, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: "辞書", start: 75, end: 81, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: "利用", start: 84, end: 90, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: "可能", start: 90, end: 96, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```

## 設定ファイル

Lindera は YAML 形式の設定ファイルを読み込めます。以下のファイルのパスを環境変数 `LINDERA_CONFIG_PATH` に指定すると、トークナイザーの動作を Rust コードで書かずに設定できます。

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
      # Merge the numeral tokens only. A counter merged into the number token keeps japanese_number from converting it.
      tags:
        - "名詞,数"
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

`TokenizerBuilder::new()` は `LINDERA_CONFIG_PATH` で指定されたファイルを読み込みます。ファイルを直接読み込む場合は `TokenizerBuilder::from_file` を使います。

```rust
use std::path::PathBuf;

use lindera_analysis::tokenizer::TokenizerBuilder;
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

上記の例は以下のように実行できます。

```shell
% cargo run -p lindera-analysis --features=embed-ipadic --example=tokenize_with_config
```

実行結果は以下のとおりです。設定に含まれる `japanese_katakana_stem` フィルタにより、`ユーザー` は `ユーザ` になります。

```text
text: Ｌｉｎｄｅｒａは形態素解析ｴﾝｼﾞﾝです。ユーザー辞書も利用可能です。
token: "Lindera", start: 0, end: 21, details: Some(["名詞", "固有名詞", "組織", "*", "*", "*", "*", "*", "*"])
token: "形態素", start: 24, end: 33, details: Some(["名詞", "一般", "*", "*", "*", "*", "形態素", "ケイタイソ", "ケイタイソ"])
token: "解析", start: 33, end: 39, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "解析", "カイセキ", "カイセキ"])
token: "エンジン", start: 39, end: 54, details: Some(["名詞", "一般", "*", "*", "*", "*", "エンジン", "エンジン", "エンジン"])
token: "ユーザ", start: 63, end: 75, details: Some(["名詞", "一般", "*", "*", "*", "*", "ユーザー", "ユーザー", "ユーザー"])
token: "辞書", start: 75, end: 81, details: Some(["名詞", "一般", "*", "*", "*", "*", "辞書", "ジショ", "ジショ"])
token: "利用", start: 84, end: 90, details: Some(["名詞", "サ変接続", "*", "*", "*", "*", "利用", "リヨウ", "リヨー"])
token: "可能", start: 90, end: 96, details: Some(["名詞", "形容動詞語幹", "*", "*", "*", "*", "可能", "カノウ", "カノー"])
```

## 環境変数

### LINDERA_BUILD_DICTIONARY_CACHE_DIR

`LINDERA_BUILD_DICTIONARY_CACHE_DIR` 環境変数は、埋め込み辞書ビルドパイプラインのビルド時キャッシュディレクトリを指定します。辞書クレートの build script のみが読み取り、実行時の動作には影響しません。

設定すると、各ビルドは `$LINDERA_BUILD_DICTIONARY_CACHE_DIR/<version>-fmt<format>/`（`<version>` は辞書クレートのバージョン、`<format>` は辞書フォーマットバージョン）配下に 2 種類のファイルを保存します。

- ダウンロードした配布アーカイブ（MD5 で検証。無効なファイルは自動的に再ダウンロード）
- クレートに埋め込まれるビルド済みバイナリ辞書

フォーマットバージョンをパスに含めているのは、オンディスクレイアウトが異なるビルドが書いたキャッシュを「古いまま再利用」ではなく「キャッシュミス」にするためです。

これにより以下のメリットがあります。

- **オフラインビルド**: 一度キャッシュされれば、以降のビルドにネットワークアクセスは不要です
- **ビルドの高速化**: 有効なキャッシュがあればダウンロードと辞書ビルドがスキップされます
- **再現可能なビルド**: ビルド間での辞書バージョンの一貫性を保ちます

使用方法:

```shell
export LINDERA_BUILD_DICTIONARY_CACHE_DIR=/path/to/cache
cargo build --features=embed-ipadic
```

注意点:

- このディレクトリは自動管理されており、削除しても安全です（必要に応じて再ダウンロード・再ビルドされます）
- バージョンごとのサブディレクトリはアップグレードのたびに蓄積され、自動削除されません。古いものは自由に削除できます
- この変数を設定すると、`embed-*` feature が無効でも辞書クレートはダウンロードとビルドを実行します（キャッシュの事前準備に便利です）

> **非推奨:** 旧名 `LINDERA_DICTIONARIES_PATH` はフォールバックとして引き続き動作しますが（両方設定時は新名が優先）、将来のメジャーリリースで削除される予定です。

### LINDERA_CONFIG_PATH

`LINDERA_CONFIG_PATH` 環境変数は、トークナイザーの設定ファイル（YAML 形式）へのパスを指定します。これにより、Rust コードを変更せずにトークナイザーの動作を設定できます。

```shell
export LINDERA_CONFIG_PATH=./resources/config/lindera.yml
```

設定フォーマットの詳細は、[設定ファイル](#設定ファイル)セクションを参照してください。

### DOCS_RS

`DOCS_RS` 環境変数は、docs.rs でドキュメントをビルドする際に自動的に設定されます。この変数が検出されると、Lindera は実際の辞書データをダウンロードする代わりにダミーの辞書ファイルを作成します。これにより、ネットワークアクセスや大容量ファイルのダウンロードなしでドキュメントをビルドできます。

これは主に docs.rs 内部で使用されるものであり、通常ユーザーが設定する必要はありません。

### LINDERA_WORKDIR

`LINDERA_WORKDIR` 環境変数は、ビルドプロセス中に lindera-dictionary クレートによって自動的に設定されます。これはビルドされた辞書データファイルを含むディレクトリを指し、辞書クレートがデータファイルの場所を特定するために内部で使用されます。

この変数は自動的に設定されるため、ユーザーが変更する必要はありません。

## API リファレンス

API リファレンスは以下の URL で参照できます。

- [lindera](https://docs.rs/lindera)
- [lindera-analysis](https://docs.rs/lindera-analysis)

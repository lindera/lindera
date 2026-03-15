# ユーザー辞書

ユーザー辞書は、システム辞書と併用してカスタム単語を登録できる補助辞書です。ドメイン固有の用語、ブランド名、固有名詞、またはデフォルトのシステム辞書に含まれていない単語を登録する場合に有用です。

## CSVフォーマット

最もシンプルなユーザー辞書フォーマットは、3つのカラムを持つCSVファイルです：

```csv
<surface>,<part_of_speech>,<reading>
```

### CSV内容の例

```csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
とうきょうスカイツリー駅,カスタム名詞,トウキョウスカイツリーエキ
```

各辞書タイプ（IPADIC、UniDic、ko-dicなど）は、文脈ID、コスト、すべての素性フィールドを完全に制御できる詳細CSVフォーマットもサポートしています。各辞書タイプの詳細フォーマットについては [辞書](./dictionaries.md) セクションを参照してください。

## Rust APIの例

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

実行結果は以下のようになります：

```text
text:   東京スカイツリーの最寄り駅はとうきょうスカイツリー駅です
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,東京スカイツリー,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,とうきょうスカイツリー駅,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

## CLIでのユーザー辞書ビルド

CLIを使用して、CSVからバイナリ形式のユーザー辞書をビルドできます：

```shell
lindera build --src <source_dir> --dest <dest_dir> --metadata <metadata.json> --user
```

### バイナリとCSVのユーザー辞書

- **CSVフォーマット**: 実行時に読み込み・パースされます。開発時や小規模な辞書に便利です。
- **バイナリフォーマット**: 高速な読み込みのために事前コンパイルされます。大規模なユーザー辞書を本番環境で使用する場合に推奨されます。

どちらのフォーマットも`Segmenter`の作成時に指定できます。バイナリフォーマットはCSVのパースステップをスキップするため、起動時間が短縮されます。

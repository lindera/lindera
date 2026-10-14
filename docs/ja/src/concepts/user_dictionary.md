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

表層形（1 列目）は書いたとおりに使われます。先頭や末尾の空白も語の一部になり、空白だけの表層形も登録できます（IPADIC は U+3000 をこの形で登録しています）。読み飛ばされるのは空の表層形だけです。`lindera build` でビルドするシステム辞書の見出し語の CSV も同じです。

末尾が空白の語は、空白をラティス上で読み飛ばすとき（SudachiDict を除く同梱辞書のデフォルト。[空白文字の扱い](../lindera/segmenter.md#空白文字の扱い) を参照）も、MeCab と同じくその空白をトークンに含みます。ただし読み飛ばすときは、辞書の `SPACE` カテゴリの空白で始まる語や、その空白だけの語は一致しません。読み飛ばした空白から始まる語はないためです。U+3000 は `SPACE` カテゴリではないため、IPADIC の U+3000 の語は引き続き一致します。

各辞書タイプ（IPADIC、UniDic、ko-dicなど）は、文脈ID、コスト、すべての素性フィールドを完全に制御できる詳細CSVフォーマットもサポートしています。各辞書タイプの詳細フォーマットについては [辞書](./dictionaries.md) セクションを参照してください。

## 同点の見出し語

表層形・左右の文脈 ID・コストがすべて同じ 2 つの見出し語は同点です。どちらを使っても経路のコストは同じになります。ユーザー辞書の見出し語は、同点のシステム辞書の見出し語より優先して選ばれます。そのため、システム辞書の見出し語の表層形・文脈 ID・コストをそのまま使った詳細形式の見出し語（たとえば語に別の読みを与えるもの）は、システム辞書の見出し語の代わりに出力されます。MeCab はシステム辞書の見出し語を選びます。同点のユーザー辞書の見出し語どうしでは、MeCab と同じく先の行が選ばれます（v7 より前は後の行でした。[v6 から v7 への移行](../migration_v6_to_v7.md#mecab-と同じく同点の見出し語は-csv-で先の行を選ぶ)を参照）。簡易形式の見出し語には、辞書の `metadata.json` のコストと文脈 ID（`default_word_cost`・`default_left_context_id`・`default_right_context_id`。IPADIC では `-10000`・`0`・`0`）が使われるため、システム辞書の見出し語と同点になることはまずありません。

## 文脈 ID

詳細形式の見出し語の左右の文脈 ID は、組み合わせるシステム辞書の接続コスト行列のセルを指していなければなりません。左文脈 ID はそのシステム辞書の `matrix.def` のヘッダの 2 つ目の値より小さく、右文脈 ID は 1 つ目の値より小さくなければなりません。`lindera build` がシステム辞書自体に課すのと同じ範囲です。たとえば IPADIC の行列は 1316 x 1316 です。

CSV のユーザー辞書をシステム辞書と一緒にビルドするときは、ビルドしながら各行を確認し、行列の範囲外にある最初の行で、その CSV の行番号と表層形を示すエラーを出して失敗します。

- `--system-dict` でシステム辞書を指定した `lindera build --user`（Rust API では `DictionaryBuilder::with_system_matrix`。[文脈 ID の確認](../lindera-cli/commands.md#文脈-id-の確認) を参照）
- 設定（`Segmenter::from_config`、`TokenizerBuilder`）や `lindera tokenize --user-dict` で指定した `.csv` のユーザー辞書と、Python・Node.js・Ruby・PHP のバインディングの `TokenizerBuilder` にパスで渡した `.csv` のユーザー辞書
- 読み込み済みのシステム辞書に合わせてユーザー辞書を読み込む関数（Rust API・Python・Ruby の `load_user_dictionary_for`、Node.js の `loadUserDictionaryFor`、PHP の `Lindera\Dictionary::loadUserFor`、WebAssembly の `loadUserDictionaryFromBytesFor`）
- 読み込み済みのシステム辞書に合わせてユーザー辞書をビルドする関数（Python・Ruby の `build_user_dictionary_for`、Node.js の `buildUserDictionaryFor`、PHP の `Lindera\Dictionary::buildUserFor`）

たとえば、1 行目が IPADIC の行列の範囲内にあり、2 行目の左文脈 ID が 60000 の `userdic.csv` では次のようになります。

```text
$ echo 東京スカイツリーへ行く | lindera tokenize -d embedded://ipadic -u userdic.csv
Error: LinderaError { kind: Args, source: LinderaError(kind=Parse, source=failed to build tokenizer: LinderaError(kind=Content, source=user dictionary line 2 (surface: '東京スカイツリー'): left context id 60000 is out of range for the connection matrix (matrix.def size 1316 1316; left context ids must be below 1316)))
```

それ以外のユーザー辞書は、システム辞書なしでビルドされます。`--system-dict` を指定せずに `lindera build --user` で書き出した `.bin`、バインディングのビルド関数でメタデータだけを使って書き出した `.bin`（Python・Ruby の `build_user_dictionary`、Node.js の `buildUserDictionary`、PHP の `Lindera\Dictionary::buildUser`）、メタデータだけで読み込んだユーザー辞書（Rust API・Python・Ruby の `load_user_dictionary`、Node.js の `loadUserDictionary`、PHP の `Lindera\Dictionary::loadUser`、WebAssembly の `loadUserDictionaryFromBytes`）です。この場合、文脈 ID は 2 つの辞書を組み合わせるときに確認します。`Segmenter::try_new`、設定からの構築、`lindera tokenize --user-dict`、各言語バインディングの `Tokenizer` のコンストラクタとビルダーは、行列の範囲外にある最初の見出し語についてエラーを返します。エラーは、見出し語をその単語 ID（表層形の順に並べた行を 0 から数えた番号）と詳細フィールドで示します。上の `userdic.csv` からビルドした `.bin` では次のようになります。

```text
$ echo 東京スカイツリーへ行く | lindera tokenize -d embedded://ipadic -u user_dict/userdic.bin
Error: LinderaError { kind: Args, source: LinderaError(kind=Parse, source=failed to build tokenizer: LinderaError(kind=Content, source=user dictionary entry 1 (名詞,固有名詞,一般,*,*,*,東京スカイツリー,トウキョウスカイツリー,トウキョウスカイツリー): left context id 60000 is out of range for the connection matrix (matrix.def size 1316 1316; left context ids must be below 1316)))
```

`Segmenter::new` はこの ID を確認しません。範囲外の見出し語があると、その見出し語が入力に一致したときに解析中にパニックします。v7.0.0 より前は、どの経路もこの動作でした。簡易形式の見出し語も確認します。簡易形式の見出し語には、ビルドに使う `metadata.json` の文脈 ID（`default_left_context_id` と `default_right_context_id`。同梱のどの辞書でも 0）が使われます。

## Rust APIの例

```rust
use std::fs::File;
use std::path::PathBuf;

use lindera::dictionary::{Metadata, load_dictionary, load_user_dictionary};
use lindera::error::LinderaErrorKind;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::analysis::tokenizer::Tokenizer;
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
    // `try_new` checks the user dictionary's context IDs against the
    // dictionary's connection matrix (see "Context IDs" above).
    let segmenter = Segmenter::try_new(
        Mode::Normal,
        dictionary,
        Some(user_dictionary), // Using the loaded user dictionary
    )?;

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
token:  東京スカイツリー        カスタム名詞,*,*,*,*,*,*,トウキョウスカイツリー,*
token:  の      助詞,連体化,*,*,*,*,の,ノ,ノ
token:  最寄り駅        名詞,一般,*,*,*,*,最寄り駅,モヨリエキ,モヨリエキ
token:  は      助詞,係助詞,*,*,*,*,は,ハ,ワ
token:  とうきょうスカイツリー駅        カスタム名詞,*,*,*,*,*,*,トウキョウスカイツリーエキ,*
token:  です    助動詞,*,*,*,特殊・デス,基本形,です,デス,デス
```

`base_form`フィールド（CSV/詳細情報の7番目のフィールド）が表層形ではなく`*`になっている点に注意してください。
シンプルな3カラムのユーザー辞書スキーマでは `surface`、`part_of_speech`、`reading` の3つしか指定できず、
`base_form` を含むそれ以外のフィールドはすべて辞書の `metadata.default_field_value`
（IPADICの場合は`*`）で埋められます。

## CLIでのユーザー辞書ビルド

CLIを使用して、CSVからバイナリ形式のユーザー辞書をビルドできます：

```shell
lindera build --src <user_dictionary.csv> --dest <dest_dir> --metadata <metadata.json> --user [--system-dict <system_dictionary>]
```

`--system-dict` を指定すると、組み合わせるシステム辞書と照らし合わせて文脈 ID を確認します（[文脈 ID](#文脈-id) を参照）。

### バイナリとCSVのユーザー辞書

- **CSVフォーマット**: 実行時に読み込み・パースされます。開発時や小規模な辞書に便利です。
- **バイナリフォーマット**: 高速な読み込みのために事前コンパイルされます。大規模なユーザー辞書を本番環境で使用する場合に推奨されます。

どちらのフォーマットも`Segmenter`の作成時に指定できます。バイナリフォーマットはCSVのパースステップをスキップするため、起動時間が短縮されます。

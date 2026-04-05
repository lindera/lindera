# クイックスタート

この例では、Linderaの基本的な使い方を説明します。

以下の処理を行います：

- Normalモードでトークナイザーを作成
- 入力テキストをトークナイズ（形態素解析）
- トークンを出力

まず、[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済みIPADIC辞書をダウンロードし、ローカルディレクトリ（例: `/path/to/ipadic`）に展開してください。

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("/path/to/ipadic")?;
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

上記の例は以下のように実行できます：

```shell
% cargo run --example=tokenize
```

> [!TIP]
> `embed-ipadic` feature を使って辞書をバイナリに埋め込む場合（上級者向け）は、ファイルパスの代わりに `load_dictionary("embedded://ipadic")` を使用できます。詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

実行結果は以下のようになります：

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

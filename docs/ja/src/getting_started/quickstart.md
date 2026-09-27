# クイックスタート

この例では、Linderaの基本的な使い方を説明します。

以下の処理を行います：

- Normalモードでセグメンターを作成
- 入力テキストを分割（形態素解析）
- トークンを出力

この例では `embed-ipadic` feature を使用します。この feature はビルド時にIPADIC辞書を自動的にダウンロードしてバイナリに埋め込むため、辞書を手動でダウンロードする必要はありません。

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
% cargo run --features embed-ipadic --example=segment
```

> [!TIP]
> 辞書をバイナリに埋め込みたくない場合は、[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済みIPADIC辞書をダウンロードしてローカルディレクトリ（例: `/path/to/ipadic`）に展開し、代わりに `load_dictionary("/path/to/ipadic")` を呼び出してください。この場合、`embed-ipadic` feature は不要です。詳細は [Feature フラグ](../development/feature_flags.md) を参照してください。

実行結果は以下のようになります：

```text
text:   関西国際空港限定トートバッグ
token:  関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
token:  限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
token:  トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

> [!NOTE]
> character filter・token filter・`Tokenizer` API は `lindera::analysis`
> （デフォルトで有効な `analysis` feature）として利用できるため、
> `lindera = "7"` だけで分析チェーン全体を利用できます。追加の依存は不要です。
> `default-features = false` にするとセグメンターのみになります。詳細は
> [Lindera Analysis](../lindera-analysis.md) を、v6 からアップグレードする場合は
> [v6 から v7 への移行](../migration_v6_to_v7.md) を参照してください。

# クイックスタート

このページでは、Lindera が公開するモデルで 1 つの文を解析します。コマンドラインと、Rust の両方を示します。Lindera 7.1.0 以降が必要です。

## 辞書とモデルを手に入れる

IPADIC のモデルは、IPADIC の辞書と一緒に使います。`lindera download` で両方をインストールでき、次に渡すパスが表示されます。

```shell
% lindera download ipadic
/home/user/.local/share/lindera/dictionaries/ipadic
% lindera download dependency-ipadic
/home/user/.local/share/lindera/dictionaries/dependency-ipadic/dependency.model
```

1 つ目は辞書のディレクトリ、2 つ目はモデルファイルです。スクリプトでは、`$(lindera download <name>)` で受け取れます。

## `lindera` コマンドで

トークンフィルター `japanese_dependency` が、すべてのトークンに、その文節と、その文節の係り先の文節を書き込みます。

```shell
% DICT=$(lindera download ipadic)
% MODEL=$(lindera download dependency-ipadic)
% echo "彼は赤い花を見た。" | lindera tokenize --dict "$DICT" --output json \
    --token-filter "japanese_dependency:{\"model\":\"$MODEL\"}"
```

JSON の出力の各トークンに、`dependency` が入ります。各トークンの表層と、その `dependency` の `bunsetsu`・`head`・`role` は、次のとおりです。

| トークン | `bunsetsu` | `head` | `role` |
| --- | --- | --- | --- |
| 彼 | 0 | 3 | `head` |
| は | 0 | 3 | `function` |
| 赤い | 1 | 2 | `head_function` |
| 花 | 2 | 3 | `head` |
| を | 2 | 3 | `function` |
| 見 | 3 | `null` | `head` |
| た | 3 | `null` | `function` |
| 。 | 3 | `null` | `other` |

この文には、文節が 4 つあります。彼は (0) と 花を (2) は 見た。 (3) に係り、赤い (1) は 花を に係り、最後の文節はどこにも係りません。欄の意味、テキストを文に分ける規則、設定ファイルやバインディングからの使い方は、[トークンフィルター](./token_filter.md)にあります。

## `lindera-dependency` コマンドで

クレートのコマンドは、ラティス形式を書き出します。`cabocha -f1` が書く形式です。

```shell
% cargo install lindera-dependency --features cli
% echo "彼は赤い花を見た。" | lindera-dependency parse --model "$MODEL" --dict "$DICT"
```

```text
* 0 3D 0/1 0.000000
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/0 0.704616
赤い	形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ
* 2 3D 0/1 0.000000
花	名詞,一般,*,*,*,*,花,ハナ,ハナ
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 3 -1D 0/1 0.000000
見	動詞,自立,*,*,一段,連用形,見る,ミ,ミ
た	助動詞,*,*,*,特殊・タ,基本形,た,タ,タ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

`* 1 2D 0/0 0.704616` のような行が、文節の始まりです。文節の番号、係り先の文節の番号、主辞と機能語、その判定の点数が並びます。[コマンドライン](./command_line.md)を見てください。

## Rust から

```toml
[dependencies]
lindera = { version = "7.1", features = ["dependency"] }
```

```rust
use std::path::Path;

use lindera::analysis::token_filter::BoxTokenFilter;
use lindera::analysis::token_filter::japanese_dependency::JapaneseDependencyTokenFilter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

fn main() -> lindera::LinderaResult<()> {
    // The paths that `lindera download ipadic` and
    // `lindera download dependency-ipadic` print.
    let mut args = std::env::args().skip(1);
    let dictionary = args.next().expect("the directory of the dictionary");
    let model = args.next().expect("the model file");

    let dictionary = load_dictionary(&dictionary)?;
    let mut tokenizer = Tokenizer::new(Segmenter::new(Mode::Normal, dictionary, None));
    let filter = JapaneseDependencyTokenFilter::from_path(Path::new(&model))?;
    tokenizer.append_token_filter(BoxTokenFilter::from(filter));

    for token in tokenizer.tokenize("彼は赤い花を見た。")? {
        if let Some(dependency) = token.dependency() {
            // The bunsetsu of the token, and the bunsetsu that one depends on.
            println!(
                "{}\t{}\t{:?}\t{}",
                token.surface,
                dependency.bunsetsu,
                dependency.head,
                dependency.role.as_str()
            );
        }
    }
    Ok(())
}
```

2 つのパスを引数に渡すと、次のように表示します。

```text
彼	0	Some(3)	head
は	0	Some(3)	function
赤い	1	Some(2)	head_function
花	2	Some(3)	head
を	2	Some(3)	function
見	3	None	head
た	3	None	function
。	3	None	other
```

Lindera のものでない形態素を解析する方法や、文節の区切りと解析器を単体で使う方法は、[係り受けの解析](./parsing.md)と[モデルファイル](./model_file.md)にあります。

結果が何を表しているのか、ほかの種類の文ではどうなるか、結果を使うプログラムは、[結果の読み方と使い方](./reading_the_result.md)にあります。

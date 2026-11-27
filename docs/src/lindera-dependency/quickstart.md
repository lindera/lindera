# Quick Start

This page parses one sentence with the model that Lindera releases, from the command line and from Rust. It needs Lindera 7.1.0 or later.

## Getting the dictionary and the model

The model for IPADIC goes with the IPADIC dictionary. `lindera download` installs both, and prints the path to pass on:

```shell
% lindera download ipadic
/home/user/.local/share/lindera/dictionaries/ipadic
% lindera download dependency-ipadic
/home/user/.local/share/lindera/dictionaries/dependency-ipadic/dependency.model
```

The first is the directory of the dictionary, the second the model file. `$(lindera download <name>)` gives each to a script.

## With the `lindera` command

The token filter `japanese_dependency` writes on every token its bunsetsu and the bunsetsu that one depends on:

```shell
% DICT=$(lindera download ipadic)
% MODEL=$(lindera download dependency-ipadic)
% echo "彼は赤い花を見た。" | lindera tokenize --dict "$DICT" --output json \
    --token-filter "japanese_dependency:{\"model\":\"$MODEL\"}"
```

Each token of the JSON output has a `dependency`. The surface of each token with the `bunsetsu`, the `head` and the `role` of its `dependency`:

| Token | `bunsetsu` | `head` | `role` |
| --- | --- | --- | --- |
| 彼 | 0 | 3 | `head` |
| は | 0 | 3 | `function` |
| 赤い | 1 | 2 | `head_function` |
| 花 | 2 | 3 | `head` |
| を | 2 | 3 | `function` |
| 見 | 3 | `null` | `head` |
| た | 3 | `null` | `function` |
| 。 | 3 | `null` | `other` |

The sentence has four bunsetsu: 彼は (0) and 花を (2) depend on 見た。 (3), 赤い (1) on 花を, and the last one on nothing. [Token Filter](./token_filter.md) describes the fields, how a text is cut into sentences, and the same from a configuration file and from the bindings.

## With the `lindera-dependency` command

The command of the crate writes the lattice format, which is what `cabocha -f1` writes:

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

A line `* 1 2D 0/0 0.704616` starts a bunsetsu: its number, the number of the bunsetsu it depends on, its head word and function word, and the score of the decision. See [Command Line](./command_line.md).

## From Rust

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

With the two paths as its arguments, it prints:

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

To parse morphemes that do not come from Lindera, or to use the chunker and the parser on their own, see [Parsing](./parsing.md) and [Model File](./model_file.md).

For what the result says, on more kinds of sentence, and for a program that uses it, see [Reading and Using the Result](./reading_the_result.md).

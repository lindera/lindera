# Examples

This page shows tokenization examples using the SudachiDict dictionary.

## Tokenize with external SudachiDict

```shell
% echo "令和五年に始まった。推し活が楽しい。" | lindera tokenize \
  --dict /tmp/lindera-sudachidict
```

```text
令和	令和,名詞,固有名詞,一般,*,*,*,レイワ,令和,*,A,*,*,*,*
五	五,名詞,数詞,*,*,*,*,ゴ,五,*,A,*,*,*,017040
年	年,名詞,普通名詞,助数詞可能,*,*,*,ネン,年,*,A,*,*,*,*
に	に,助詞,格助詞,*,*,*,*,ニ,に,*,A,*,*,*,*
始まっ	始まっ,動詞,一般,*,*,五段-ラ行,連用形-促音便,ハジマッ,始まる,380965,A,*,*,*,000378
た	た,助動詞,*,*,*,助動詞-タ,終止形-一般,タ,た,83558,A,*,*,*,*
。	。,補助記号,句点,*,*,*,*,。,。,*,A,*,*,*,*
推し活	推し活,名詞,普通名詞,一般,*,*,*,オシカツ,推し活,*,A,*,*,*,*
が	が,助詞,格助詞,*,*,*,*,ガ,が,*,A,*,*,*,*
楽しい	楽しい,形容詞,一般,*,*,形容詞,終止形-一般,タノシイ,楽しい,519727,A,*,*,*,*
。	。,補助記号,句点,*,*,*,*,。,。,*,A,*,*,*,*
EOS
```

Notice that `令和` is a single proper noun (IPADIC and UniDic 2.1.2 split it into `令` and `和`) and `推し活` is in-vocabulary. The first detail column is the display surface, followed by the part-of-speech columns and SudachiDict-specific columns (normalized form, split mode, synonym group IDs).

## Tokenize with embedded SudachiDict

```shell
% echo "令和五年に始まった。推し活が楽しい。" | lindera tokenize \
  --dict embedded://sudachidict
```

The output is the same as above.

NOTE: To include SudachiDict dictionary in the binary, you must build with the `--features=embed-sudachidict` option.

## Rust API example

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera_analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://sudachidict")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "令和五年に始まった。推し活が楽しい。";
    let mut tokens = tokenizer.tokenize(text)?;
    for token in tokens.iter_mut() {
        // Schema-aware access: works regardless of column positions
        let pos = token.get("part_of_speech").unwrap_or_default().to_string();
        let details = token.details().join(",");
        println!("{}\t{}\t{}", token.surface.as_ref(), pos, details);
    }
    Ok(())
}
```

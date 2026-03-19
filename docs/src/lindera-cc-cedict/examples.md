# Examples

## Tokenize with external CC-CEDICT

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-cc-cedict-0.1.0-20200409
```

```text
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
中文    *,*,*,*,Zhong1 wen2,中文,中文,Chinese language/
形态学  *,*,*,*,xing2 tai4 xue2,形態學,形态学,morphology (in biology or linguistics)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
。      *,*,*,*,*,*,*,*
EOS
```

## Tokenize with embedded CC-CEDICT

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://cc-cedict
```

```text
可以    *,*,*,*,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good/
进行    *,*,*,*,jin4 xing2,進行,进行,to advance/to conduct/underway/in progress/to do/to carry out/to carry on/to execute/
中文    *,*,*,*,Zhong1 wen2,中文,中文,Chinese language/
形态学  *,*,*,*,xing2 tai4 xue2,形態學,形态学,morphology (in biology or linguistics)/
分析    *,*,*,*,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4]/
。      *,*,*,*,*,*,*,*
EOS
```

NOTE: To include CC-CEDICT dictionary in the binary, you must build with the `--features=embed-cc-cedict` option.

## Rust API example

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://cc-cedict")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "可以进行中文形态学分析。";
    let mut tokens = tokenizer.tokenize(text)?;
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }
    Ok(())
}
```

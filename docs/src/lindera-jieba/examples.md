# Examples

## Tokenize with external Jieba

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict /tmp/lindera-jieba-0.1.1
```

```text
可以    c,CHINESE,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good,2,可,以,high
进行    v,CHINESE,jin4 xing2,進行,进行,(of a process etc) to proceed; to be in progress; to be underway/(of people) to carry out; to conduct (an investigation or discussion etc)/(of an army etc) to be on the march; to advance,2,进,行,high
中文    nz,CHINESE,Zhong1 wen2,中文,中文,Chinese language,2,中,文,high
形态    n,CHINESE,xing2 tai4,形態,形态,shape/form/pattern/morphology,2,形,态,high
学      n,CHINESE,xue2,學,学,to learn/to study/to imitate/science/-ology,1,学,学,high
分析    vn,CHINESE,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4],2,分,析,high
。      w,*,*,*,*,*,*,*,*,*
EOS
```

## Tokenize with embedded Jieba

```shell
% echo "可以进行中文形态学分析。" | lindera tokenize \
  --dict embedded://jieba
```

```text
可以    c,CHINESE,ke3 yi3,可以,可以,can/may/possible/able to/not bad/pretty good,2,可,以,high
进行    v,CHINESE,jin4 xing2,進行,进行,(of a process etc) to proceed; to be in progress; to be underway/(of people) to carry out; to conduct (an investigation or discussion etc)/(of an army etc) to be on the march; to advance,2,进,行,high
中文    nz,CHINESE,Zhong1 wen2,中文,中文,Chinese language,2,中,文,high
形态    n,CHINESE,xing2 tai4,形態,形态,shape/form/pattern/morphology,2,形,态,high
学      n,CHINESE,xue2,學,学,to learn/to study/to imitate/science/-ology,1,学,学,high
分析    vn,CHINESE,fen1 xi1,分析,分析,to analyze/analysis/CL:個|个[ge4],2,分,析,high
。      w,*,*,*,*,*,*,*,*,*
EOS
```

NOTE: To include Jieba dictionary in the binary, you must build with the `--features=embed-jieba` option.

## Rust API example

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://jieba")?;
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

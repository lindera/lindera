# Examples

This page shows tokenization examples using the UniDic CWJ dictionary.

## Tokenize with external UniDic CWJ

Download the dictionary with `lindera download unidic-cwj`, or [build it](./build.md), and pass its directory:

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-unidic-cwj-202512
```

```text
日本	名詞,固有名詞,地名,国,*,*,ニッポン,日本,日本,ニッポン,日本,ニッポン,固,*,*,*,*,*,*,国,ニッポン,ニッポン,ニッポン,ニッポン,3,*,*,7821659499274752,28455
語	名詞,普通名詞,一般,*,*,*,ゴ,語,語,ゴ,語,ゴ,漢,*,*,*,*,*,*,体,ゴ,ゴ,ゴ,ゴ,1,C3,*,3665230634820096,13334
の	助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*,*,*,格助,ノ,ノ,ノ,ノ,*,名詞%F1,*,7968444268028416,28989
形態	名詞,普通名詞,一般,*,*,*,ケイタイ,形態,形態,ケータイ,形態,ケータイ,漢,*,*,*,*,*,*,体,ケイタイ,ケイタイ,ケイタイ,ケイタイ,0,C2,*,3024215389381120,11002
素	接尾辞,名詞的,一般,*,*,*,ソ,素,素,ソ,素,ソ,漢,*,*,*,*,*,*,接尾体,ソ,ソ,ソ,ソ,*,C3,*,5752103704338944,20926
解析	名詞,普通名詞,サ変可能,*,*,*,カイセキ,解析,解析,カイセキ,解析,カイセキ,漢,*,*,*,*,*,*,体,カイセキ,カイセキ,カイセキ,カイセキ,0,C2,*,1590177315299840,5785
を	助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*,*,*,格助,ヲ,ヲ,ヲ,ヲ,*,動詞%F2@0,名詞%F1,形容詞%F2@-1,*,11381878116459008,41407
行う	動詞,一般,*,*,五段-ワア行,連体形-一般,オコナウ,行う,行う,オコナウ,行う,オコナウ,和,*,*,*,*,*,*,用,オコナウ,オコナウ,オコナウ,オコナウ,0,C2,*,1340038453535425,4875
こと	名詞,普通名詞,一般,*,*,*,コト,事,こと,コト,こと,コト,和,コ濁,基本形,*,*,*,B1WS2WB3WB6SjShS,体,コト,コト,コト,コト,2,C3,*,3528350027096576,12836
が	助詞,格助詞,*,*,*,*,ガ,が,が,ガ,が,ガ,和,*,*,*,*,*,*,格助,ガ,ガ,ガ,ガ,*,動詞%F2@0,名詞%F1,*,2168520431510016,7889
でき	動詞,非自立可能,*,*,上一段-カ行,連用形-一般,デキル,出来る,でき,デキ,できる,デキル,和,*,*,*,*,*,*,用,デキ,デキル,デキ,デキル,2,C1,M4@1,7043480111161985,25624
ます	助動詞,*,*,*,助動詞-マス,終止形-一般,マス,ます,ます,マス,ます,マス,和,*,*,*,*,*,*,助動,マス,マス,マス,マス,*,動詞%F4@1,*,9812325267808939,35697
。	補助記号,句点,*,*,*,*,*,。,。,*,。,*,記号,*,*,*,*,*,*,補助,*,*,*,*,*,*,*,6880571302400,25
EOS
```

The segmentation follows UniDic's short units, as UniDic 2.1.2's does ("日本" and "語", "形態" and "素"), and each token carries the 33 fields. The accent connection type of "を" holds commas (`動詞%F2@0,名詞%F1,形容詞%F2@-1`): it is one field, quoted in `lex.csv`.

## Tokenize with embedded UniDic CWJ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://unidic-cwj
```

The embedded dictionary is built from the same files, so the output is the same as above.

NOTE: To include the UniDic CWJ dictionary in the binary, build with the `--features=embed-unidic-cwj` option. The binary then carries the 1.2 GB dictionary.

## Rust API example

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://unidic-cwj")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "日本語の形態素解析を行うことができます。";
    let mut tokens = tokenizer.tokenize(text)?;
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }
    Ok(())
}
```

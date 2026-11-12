# 使用例

このページでは、UniDic CSJ 辞書を使用したトークナイズの例を示します。

## 外部 UniDic CSJ でトークナイズ

`lindera download unidic-csj` で辞書を取得するか、[ビルド](./build.md)して、そのディレクトリを渡します:

```shell
% echo "そんなら君の指を切ってみろ。" | lindera tokenize \
  --dict /tmp/lindera-unidic-csj-202512
```

```text
そんな	形状詞,一般,*,*,*,*,ソンナ,そんな,そんな,ソンナ,そんな,ソンナ,和,*,*,*,*,*,*,相,ソンナ,ソンナ,ソンナ,ソンナ,0,C2,*,5912632401994240,21510
ら	接尾辞,名詞的,一般,*,*,*,ラ,等,ら,ラ,ら,ラ,和,*,*,*,*,*,*,接尾体,ラ,ラ,ラ,ラ,*,C3,*,10877477157216768,39572
君	接尾辞,名詞的,一般,*,*,*,クン,君,君,クン,君,クン,漢,*,*,*,*,*,*,接尾体,クン,クン,クン,クン,*,C4,*,2928557877764608,10654
の	助詞,格助詞,*,*,*,*,ノ,の,の,ノ,の,ノ,和,*,*,*,*,*,*,格助,ノ,ノ,ノ,ノ,*,名詞%F1,*,7968444268028416,28989
指	名詞,普通名詞,一般,*,*,*,ユビ,指,指,ユビ,指,ユビ,和,*,*,*,*,*,*,体,ユビ,ユビ,ユビ,ユビ,2,C3,*,10688361157239296,38884
を	助詞,格助詞,*,*,*,*,ヲ,を,を,オ,を,オ,和,*,*,*,*,*,*,格助,ヲ,ヲ,ヲ,ヲ,*,動詞%F2@0,名詞%F1,形容詞%F2@-1,*,11381878116459008,41407
切っ	動詞,非自立可能,*,*,五段-ラ行,連用形-促音便,キル,切る,切っ,キッ,切る,キル,和,キ濁,基本形,*,*,*,*,用,キッ,キル,キッ,キル,1,C1,*,2626192180126340,9554
て	助詞,接続助詞,*,*,*,*,テ,て,て,テ,て,テ,和,*,*,*,*,*,*,接助,テ,テ,テ,テ,*,動詞%F1,形容詞%F2@-1,*,6837321680953856,24874
みろ	動詞,非自立可能,*,*,上一段-マ行,命令形,ミル,見る,みろ,ミロ,みる,ミル,和,*,*,*,*,*,*,用,ミロ,ミル,ミロ,ミル,1,C1,*,10148500948001573,36920
。	補助記号,句点,*,*,*,*,*,。,。,*,。,*,記号,*,*,*,*,*,*,補助,*,*,*,*,*,*,*,6880571302400,25
EOS
```

項目は UniDic CWJ と同じですが、コストが違うので、同じテキストでも分かれ方が違うことがあります。UniDic CSJ は「そんなら」を「そんな」と「ら」に分け、UniDic CWJ は「そん」と「なら」に分けます。

## 埋め込み UniDic CSJ でトークナイズ

```shell
% echo "そんなら君の指を切ってみろ。" | lindera tokenize \
  --dict embedded://unidic-csj
```

埋め込みの辞書は同じファイルから作られるので、出力は上と同じです。

注意: UniDic CSJ 辞書をバイナリに含めるには、`--features=embed-unidic-csj` オプションを付けてビルドする必要があります。実行ファイルは 1.2 GB の辞書を含みます。

## Rust API の使用例

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::analysis::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://unidic-csj")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "そんなら君の指を切ってみろ。";
    let mut tokens = tokenizer.tokenize(text)?;
    for token in tokens.iter_mut() {
        let details = token.details().join(",");
        println!("{}\t{}", token.surface.as_ref(), details);
    }
    Ok(())
}
```

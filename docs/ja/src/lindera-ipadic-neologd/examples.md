# 使用例

このページでは、IPADIC NEologd 辞書を使用したトークナイズの例を示します。

## 外部 IPADIC NEologd でトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict /tmp/lindera-ipadic-neologd-0.0.7-20200820
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

NEologd では「形態素解析」が単一の複合名詞として扱われますが、標準の IPADIC では「形態素」と「解析」に分割されます。

## 埋め込み IPADIC NEologd でトークナイズ

```shell
% echo "日本語の形態素解析を行うことができます。" | lindera tokenize \
  --dict embedded://ipadic-neologd
```

```text
日本語  名詞,一般,*,*,*,*,日本語,ニホンゴ,ニホンゴ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
形態素解析      名詞,固有名詞,一般,*,*,*,形態素解析,ケイタイソカイセキ,ケイタイソカイセキ
を      助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
行う    動詞,自立,*,*,五段・ワ行促音便,基本形,行う,オコナウ,オコナウ
こと    名詞,非自立,一般,*,*,*,こと,コト,コト
が      助詞,格助詞,一般,*,*,*,が,ガ,ガ
でき    動詞,自立,*,*,一段,連用形,できる,デキ,デキ
ます    助動詞,*,*,*,特殊・マス,基本形,ます,マス,マス
。      記号,句点,*,*,*,*,。,。,。
EOS
```

注意: IPADIC NEologd 辞書をバイナリに含めるには、`--features=embed-ipadic-neologd` オプションを付けてビルドする必要があります。

## Rust API の使用例

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic-neologd")?;
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

# 使用例

このページでは、SudachiDict 辞書を使用したトークナイズの例を示します。

## 外部 SudachiDict でトークナイズ

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

`令和` が単一の固有名詞になり（IPADIC と UniDic 2.1.2 は `令` と `和` に分割します）、`推し活` が語彙に含まれている点に注目してください。詳細（details）の先頭カラムは見出し（解析結果表示用）で、その後に品詞カラムと SudachiDict 固有のカラム（正規化表記、分割タイプ、同義語グループID）が続きます。

## 埋め込み SudachiDict でトークナイズ

```shell
% echo "令和五年に始まった。推し活が楽しい。" | lindera tokenize \
  --dict embedded://sudachidict
```

出力は上記と同じです。

注意: SudachiDict 辞書をバイナリに含めるには、`--features=embed-sudachidict` オプションを付けてビルドする必要があります。

## Rust API の使用例

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

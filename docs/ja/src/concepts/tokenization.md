# トークナイズ

Linderaは複数のトークナイズモードを提供し、代替の分割候補を列挙するためのN-Best解析をサポートしています。

## トークナイズモード

### Normalモード

Normalモードは辞書エントリに基づく標準的なトークナイズを実行します。辞書に単一エントリとして存在する複合語はそのまま保持されます。

**例** -- "関西国際空港限定トートバッグ" をNormalモードでトークナイズ：

```text
関西国際空港 | 限定 | トートバッグ
```

複合名詞 "関西国際空港"（Kansai International Airport）は辞書に1つのエントリとして存在するため、単一トークンとして保持されます。

### Decomposeモード

Decomposeモードは、複合語が辞書エントリとして存在する場合でも、さらに構成要素に分解します。

**例** -- "関西国際空港限定トートバッグ" をDecomposeモードでトークナイズ：

```text
関西 | 国際 | 空港 | 限定 | トートバッグ
```

複合語 "関西国際空港" は "関西"、"国際"、"空港" に分解されます。

### モードの選択

Rustでは、`Segmenter`の作成時にモードを指定します：

```rust
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::dictionary::load_dictionary;

let dictionary = load_dictionary("embedded://ipadic")?;

// Normal mode
let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

// Decompose mode
let segmenter = Segmenter::new(Mode::Decompose, dictionary, None);
```

CLIでは、`--mode`フラグを使用します：

```shell
echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic --mode normal
echo "関西国際空港限定トートバッグ" | lindera tokenize --dict embedded://ipadic --mode decompose
```

## N-Bestトークナイズ

N-Bestトークナイズは、総パスコスト順（低コスト = より良い分割）に上位N件のトークナイズ候補を列挙します。最良の結果が曖昧な場合や、入力テキストの代替解釈を探索したい場合に有用です。

### アルゴリズム

N-Bestトークナイズは**Forward-DP Backward-A\***アルゴリズムに基づいており、MeCabのN-Best実装と互換性があります。フォワードパスは動的計画法で最適コストを計算し、バックワードパスはA\*探索を使用して総コストの昇順にパスを列挙します。

### パラメータ

`tokenize_nbest`メソッドは以下のパラメータを受け付けます：

| パラメータ | 型 | 説明 |
| --- | --- | --- |
| `text` | `&str` | トークナイズするテキスト。 |
| `n` | `usize` | 返すN-best結果の数。 |
| `unique` | `bool` | `true`の場合、同じ単語境界位置を生成する結果を重複排除します。 |
| `cost_threshold` | `Option<i64>` | `Some(threshold)`の場合、`best_cost + threshold`以内のコストのパスのみを返します。 |

### Rust APIの例

```rust
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use lindera::tokenizer::Tokenizer;
use lindera::LinderaResult;

fn main() -> LinderaResult<()> {
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let tokenizer = Tokenizer::new(segmenter);

    let text = "すもももももももものうち";

    // Get top 3 tokenization results
    let results = tokenizer.tokenize_nbest(text, 3, false, None)?;

    for (rank, (tokens, cost)) in results.iter().enumerate() {
        println!("--- NBEST {} (cost={}) ---", rank + 1, cost);
        for token in tokens {
            let details = token.details().join(",");
            println!("{}\t{}", token.surface.as_ref(), details);
        }
    }

    Ok(())
}
```

実行結果は以下のようになります：

```text
--- NBEST 1 (cost=7546) ---
すもも  名詞,一般,*,*,*,*,すもも,スモモ,スモモ
も      助詞,係助詞,*,*,*,*,も,モ,モ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
も      助詞,係助詞,*,*,*,*,も,モ,モ
もも    名詞,一般,*,*,*,*,もも,モモ,モモ
の      助詞,連体化,*,*,*,*,の,ノ,ノ
うち    名詞,非自立,副詞可能,*,*,*,うち,ウチ,ウチ
--- NBEST 2 (cost=7914) ---
...
```

### CLIの例

```shell
echo "すもももももももものうち" | lindera tokenize --dict embedded://ipadic -N 3
```

### Latticeの再利用

繰り返しトークナイズを行う場合、`Lattice`を再利用してメモリ割り当てを削減できます：

```rust
use lindera_dictionary::viterbi::Lattice;

let mut lattice = Lattice::default();
let results = tokenizer.tokenize_nbest_with_lattice(text, &mut lattice, 3, false, None)?;
```

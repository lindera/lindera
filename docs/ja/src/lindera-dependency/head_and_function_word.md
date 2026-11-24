# 主辞と機能語

文節には、主辞（最後の内容語）と機能語（最後の助詞か助動詞）があります。読ん / で / いる / 。 では、主辞は 読ん、機能語は いる です。

- 主辞は、文節の対を索引に入れたり、三つ組を取り出したりするときに、文節の代表にする語です。
- 解析器は、素性の大半をこの 2 つから作ります。
- ラティス形式は、文節の行にその位置を持ちます（`0/2`）。

どの形態素がそれに当たるかは辞書で決まり、品詞や原形がどの列にあるかも辞書で決まります。`Schema` がそのすべてをデータとして持ち、1 つのコードがそれを当てはめます。

## 位置

```rust
use lindera_dependency::Schema;
use lindera_dependency::lattice::LatticeMorpheme;

let schema = Schema::ipadic();
// 利用 / し / た / 。
let bunsetsu = [
    LatticeMorpheme::new("利用", "名詞,サ変接続,*,*,*,*,利用,リヨウ,リヨー"),
    LatticeMorpheme::new("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
    LatticeMorpheme::new("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];
// The head word is 利用 and the function word た: (0, 2).
assert_eq!(schema.positions(&bunsetsu), Some((0, 2)));
```

`Schema::positions` は、1 つの文節の形態素を受け取り、主辞と機能語の位置を返します。位置は文節の先頭の形態素から数え、形態素がなければ `None` です。

## 規則

規則は CaboCha のもの（`findHead`）です。

1. どちらの位置も、文節の先頭の形態素から始めます。
2. 左から見て、機能語のパターンに合う形態素が機能語になります。どのパターンにも合わず、記号でもなく、「主辞にならないもの」の一覧にも無い形態素が主辞になります。
3. 主辞が機能語より後ろにあれば、機能語も主辞の位置にします。

したがって、機能語のない文節は両方とも主辞の位置になり（本 だけの文節は `0/0`）、記号だけの文節は両方とも先頭の形態素になります。

パターンは、品詞の階層の前方一致（動詞, 非自立）で、取りうる原形と、文節の中の直前の形態素のパターンを足せます。スキーマが挙げる列より素性の列が少ない形態素は、足りない列についてはどのパターンにも合いません。panic はしません。

## 句読点と括弧

スキーマは、解析器の素性のために、記号も見分けます。

```rust
assert!(schema.is_period(&bunsetsu[3]));
assert!(schema.is_symbol(&bunsetsu[3]));
// A half-width comma is an unknown word of IPADIC, and still a comma.
let comma = LatticeMorpheme::new(",", "名詞,サ変接続,*,*,*,*,*,*,*");
assert!(schema.is_comma(&comma));
```

| メソッド | 見分けるもの |
| --- | --- |
| `is_comma` | 、 と `,` `，` |
| `is_period` | 。 と `.` `．` |
| `is_open_bracket`、`is_close_bracket` | 括弧と引用符。全角と半角 |
| `is_symbol` | 上のどれかと、そのほかの記号 |

記号は、品詞でも表層でも見分けます。表層が要るのは、辞書が記号にいつも記号の品詞を付けるとは限らないためです。IPADIC には半角の記号がなく、`,` は未知語になって 名詞,サ変接続 として出ます。

## IPADIC のスキーマ

`Schema::ipadic()` は、IPADIC の列（品詞の 4 階層、活用型、活用形、原形）を挙げ、CaboCha の IPA のパターンから出発します。

| | パターン |
| --- | --- |
| 機能語 | 助詞、助動詞、動詞,非自立、動詞,接尾、形容詞,非自立、形容詞,接尾（CaboCha）。する・できる・出来る（動詞,自立） |
| 主辞にならないもの | 記号（CaboCha）と、表層で見分けた記号。名詞,非自立（こと、の、よう）。接尾辞の 的（名詞,接尾,形容動詞語幹）と さ（名詞,接尾,特殊）。助詞の直後の 動詞,自立（こと / に / なっ / た、かも / しれ / ない） |

CaboCha のパターンに足したものは、測定から決めました。UD Japanese-GSD は各文節の内容の主辞に印を付けていて、[`convert`](./training_data.md) が、スキーマの主辞がその語と一致する回数を数えます。

| 規則 | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| CaboCha の IPA の規則 | 85.43% | 86.30% | 84.61% | 86.70% |
| `Schema::ipadic()` | 96.55% (54,449 / 56,396) | 96.96% (4,021 / 4,147) | 96.92% (4,370 / 4,509) | 97.46% (9,593 / 9,843) |

最大の違いは、サ変名詞 + する（利用 / し / た）です。CaboCha の規則は し を主辞にし、コーパスは名詞を主辞にします。2 つの行の間の段階と、採らなかった変種は、[クレートの README](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#agreement-with-ud-japanese-gsd) にあります。

機能語は、スキーマだけで定義します。コーパスには、同じ意味の印がありません。

## ほかの辞書

スキーマは、フィールドが公開されている値です。列、`function_words`、`not_heads`、`symbols` と、[解析器](./parsing.md)が読む 2 つの一覧 `particles`・`lexicalized` があります。ほかの辞書のスキーマは、新しいコードを書かずに、データから作れます。このクレートが今のところ用意しているのは IPADIC だけで、UniDic が続く予定です。

```rust
use lindera_dependency::{Marks, Pattern, Schema, Symbols};

// A dictionary with two levels of part of speech and the base form in the
// third column.
let schema = Schema {
    part_of_speech: 0..2,
    base_form: Some(2),
    function_words: vec![Pattern::new(&["助詞"]), Pattern::new(&["助動詞"])],
    not_heads: vec![Pattern::new(&["動詞"]).after(Pattern::new(&["助詞"]))],
    symbols: Symbols {
        others: Marks::new(&[&["記号"]], "!?"),
        ..Symbols::default()
    },
    ..Schema::default()
};
```

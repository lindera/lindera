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

## UniDic のスキーマ

`Schema::unidic()` は、Lindera の 3 つの UniDic の辞書、`unidic`（unidic-mecab 2.1.2）・`unidic-cwj`・`unidic-csj` のスキーマです。スキーマが読む列は、3 つとも同じ位置にあります。品詞の 4 階層、活用型、活用形と、8 列目の語彙素で、語彙素を原形として使います。そのため、こと の原形は 事 です。CaboCha の UniDic のパターンから出発します。

| | パターン |
| --- | --- |
| 機能語 | 助詞、助動詞、接尾辞,形容詞的、接尾辞,形状詞的、接尾辞,動詞的（CaboCha）。動詞,非自立可能 と 形容詞,非自立可能 |
| 主辞にならないもの | 空白、補助記号、記号。に・と・を の直後の 動詞（事故 / に / よっ / て）。助動詞語幹（よう、そう、みたい）。動詞・助動詞・形容詞の直後の形式名詞（こと、ため、もの、際、うえ、ところ）。形容詞の直後の名詞的な接尾辞（接尾辞,名詞的。大き / さ） |

```rust
use lindera_dependency::Schema;
use lindera_dependency::lattice::LatticeMorpheme;

let schema = Schema::unidic();
// 利用 / する / こと / が
let bunsetsu = [
    LatticeMorpheme::new("利用", "名詞,普通名詞,サ変可能,*,*,*,リヨウ,利用"),
    LatticeMorpheme::new("する", "動詞,非自立可能,*,*,サ行変格,連体形-一般,スル,為る"),
    LatticeMorpheme::new("こと", "名詞,普通名詞,一般,*,*,*,コト,事"),
    LatticeMorpheme::new("が", "助詞,格助詞,*,*,*,*,ガ,が"),
];
// The head word is 利用 and the function word が: する is 非自立可能,
// and こと is a formal noun after a verb.
assert_eq!(schema.positions(&bunsetsu), Some((0, 3)));
```

足した規則は、IPADIC と同じように測って決めました。UniDic CWJ での値で、各行は前の行に規則を 1 つ足したものです。最後の行が `Schema::unidic()` です。

| 規則 | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| CaboCha の UniDic の規則 | 82.50% | 82.81% | 83.96% | 81.75% |
| 非自立可能 を機能語にする | 94.73% | 94.64% | 94.70% | 94.47% |
| に・と・を の直後の動詞を主辞にしない | 96.71% | 96.53% | 96.43% | 96.73% |
| 助動詞語幹 を主辞にしない | 97.19% | 97.05% | 97.10% | 97.15% |
| 用言の直後の形式名詞を主辞にしない | 97.63% | 97.49% | 97.69% | 98.04% |
| 形容詞の直後の名詞的な接尾辞を主辞にしない | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |

同じスキーマを 3 つの辞書で測った値です。

| 辞書 | GSD train | GSD dev | GSD test | PUD |
| --- | --- | --- | --- | --- |
| `unidic` | 97.52% (55,320 / 56,724) | 97.60% (4,074 / 4,174) | 97.86% (4,440 / 4,537) | 97.84% (9,676 / 9,890) |
| `unidic-cwj` | 97.70% (55,460 / 56,763) | 97.61% (4,076 / 4,176) | 97.95% (4,451 / 4,544) | 98.16% (9,707 / 9,889) |
| `unidic-csj` | 97.55% (55,366 / 56,759) | 97.49% (4,071 / 4,176) | 97.91% (4,448 / 4,543) | 97.91% (9,682 / 9,889) |

- **最も大きいのは 非自立可能 です。** UniDic は する・いる・ある・なる に 非自立可能 を付けます。CaboCha のパターンはこれらを主辞にしますが、コーパスは 利用 / する の名詞を主辞にします。
- **採らなかった規則。** 名詞的な接尾辞をどこでも主辞にしないと、GSD train は 91.14% (51,736 / 56,763) に下がります。コーパスは 長 や 内 のような接尾辞を主辞にするためです。数詞の直後に限っても 96.75% (54,917 / 56,763) に下がります。UniDic は英字 1 文字に 記号,文字 を付けます。これを記号から外すと 97.50% (55,346 / 56,763) になります。
- **スキーマで表せないもの。** コーパスは、数の後ろのパーセント記号を内容の主辞にします（3 / %。GSD train で 65 文節）。UniDic はこれに、・ と同じ 補助記号,一般 を付けます。スキーマは、記号を品詞か文字のどちらかで見分け、両方を組み合わせては見分けません。そのため、パーセント記号は記号のままで、数が主辞になります。IPADIC では `%` は名詞で、主辞になります。
- **残りの大半は、規則の問題ではありません。** コーパスの内容の主辞は、その語の最初の文字を持つ形態素に写します。コーパスの 1 語を辞書が分ける場合（数字で書いた数、カタカナの名前、k / m）は、最後の形態素を主辞にするスキーマと食い違います。

## ほかの辞書

スキーマは、フィールドが公開されている値です。列、`function_words`、`not_heads`、`symbols` と、[解析器](./parsing.md)が読む 2 つの一覧 `particles`・`lexicalized` があります。ほかの辞書のスキーマは、新しいコードを書かずに、データから作れます。このクレートは、`Schema::ipadic()` と `Schema::unidic()` の 2 つを用意しています。`Schema::for_dictionary` は、Lindera の辞書の名前からスキーマを返し、スキーマの無い辞書には `None` を返します。

```rust
use lindera_dependency::Schema;

assert_eq!(Schema::for_dictionary("ipadic"), Some(Schema::ipadic()));
// The three UniDic dictionaries have one schema.
assert_eq!(Schema::for_dictionary("unidic-cwj"), Some(Schema::unidic()));
assert_eq!(Schema::for_dictionary("ko-dic"), None);
assert_eq!(
    Schema::DICTIONARIES,
    ["ipadic", "unidic", "unidic-cwj", "unidic-csj"]
);
```

自分のスキーマも、同じように値として書きます。

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

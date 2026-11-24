# 係り受けの解析

係り受けの解析は、文の文節ごとに、その文節が係る先の文節を求めます。`parser` モジュールは、CaboCha・J.DepP と同じ方法で求めます。スタックを使って文節を左から右へたどり（shift-reduce）、分類器が 1 回に 1 つの問いに答えます。

> このクレートには、**解析器の学習済みのモデルがまだありません**。今あるのは、解析のループ、素性、手や実験で作った重みを受け取る分類器、分類器の学習事例です。学習とモデルのファイルは [#1437](https://github.com/lindera/lindera/issues/1437) で追っています。

## 文を解析する手順

1 回の判定は、2 つの文節についての問いです。**係り元**（source）は**係り先の候補**（target）に係るか、を尋ねます。

1. スタックは、係り先がまだ決まっていない文節を持ちます。最初の文節から始めます。
2. 続く文節を 1 つずつ係り先の候補にし、スタックの先頭を係り元にします。
3. 分類器が「係る」と答える間、係り元の係り先を決めてスタックから外し、次の文節を見ます。
4. 断られた係り元はスタックに残り、係り先の候補をその上に積みます。
5. 最後の文節は根です。最後の文節が来たときにスタックに残っている文節は、すべて最後の文節に係ります。このとき、分類器には尋ねません。

結果は、参照実装の解析器が仮定する木になります。どの係り先も文節の右にあり、係り受けは交差せず、最後の文節が根です。判定の数は文節の数の 2 倍に届かないので、時間は文の長さに比例します。文節が 1 つか 2 つの文では、何も尋ねません。

彼は | 本を | 読んだと | 言う で、本を が 読んだと に係り、ほかの 2 つが 言う に係る場合です。

| スタック | 係り先の候補 | 判定 | 答え |
| --- | --- | --- | --- |
| 彼は | 本を | 彼は → 本を | 係らない。本を をスタックに積む |
| 彼は, 本を | 読んだと | 本を → 読んだと | 係る。本を をスタックから外す |
| 彼は | 読んだと | 彼は → 読んだと | 係らない。読んだと をスタックに積む |
| 彼は, 読んだと | 言う | 尋ねない | どちらも 言う に係る |

## 解析器を使う

`Parser` は、辞書の[スキーマ](./head_and_function_word.md)、素性の集合、分類器を受け取ります。次の分類器は手で作ったもので、重みは 1 つだけです。を で終わる文節は、次の文節に係ります。

```rust
use std::collections::HashMap;
use std::io;

use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::parser::{FeatureSet, LinearClassifier, Parser};
use lindera_dependency::{Schema, Sentence};

// SFS is the surface of the function word of the source, and DIST:1
// says that the target is the next bunsetsu.
let pairs = HashMap::from([(("SFS:を".to_string(), "DIST:1".to_string()), 2.0)]);
let classifier = LinearClassifier::new(-1.0, HashMap::new(), pairs)?;
let parser = Parser::new(Schema::ipadic(), FeatureSet::cabocha(), classifier);

// 彼は | 本を | 読んだと | 言う
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
    LatticeMorpheme::new("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
    LatticeMorpheme::new("と", "助詞,格助詞,引用,*,*,*,と,ト,ト"),
    LatticeMorpheme::new("言う", "動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ"),
];
let bunsetsu = parser.parse(&morphemes, &[0..2, 2..4, 4..7, 7..8])?;
for (index, one) in bunsetsu.iter().enumerate() {
    println!("{index} -> {:?}, score {:?}", one.head, one.score);
}

// The result is a sentence the lattice format can hold.
let sentence = Sentence::new(morphemes, bunsetsu)?;
lattice::write(&mut io::stdout(), &sentence)?;
```

<!-- markdownlint-disable MD010 -->
```text
0 -> Some(3), score Some(0.0)
1 -> Some(2), score Some(1.0)
2 -> Some(3), score Some(0.0)
3 -> None, score Some(0.0)
* 0 3D 0/1 0.000000
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1 1.000000
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 3D 0/2 0.000000
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ	助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
と	助詞,格助詞,引用,*,*,*,と,ト,ト
* 3 -1D 0/0 0.000000
言う	動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ
EOS
```
<!-- markdownlint-enable MD010 -->

文節は、形態素の範囲で渡します。[`Chunker::chunk`](./chunking.md) が返すものと同じ形です。結果の各文節には、係り先、スキーマから求めた主辞と機能語の位置、係りを決めた判定の点数が入ります。根と、根に係る文節の点数は 0 です。その判定は尋ねないためです。

## 分類器

分類器は、`Classifier` トレイトを 2 段で実装します。

- `id` は、分類器が知っている素性の文字列に番号を返し、知らない文字列には `None` を返します。知らない素性は、解析器が捨てます。解析器は、文の各文節の各素性について 1 回ずつ尋ねます。
- `score` は、1 回の判定の素性の番号に点数を付けます。点数が 0 より大きいとき、係り元は係り先の候補に係ります。

CaboCha と同じく、判定のたびに文字列を引くことはありません。

`LinearClassifier` は、今のところこのクレートが持つただ 1 つの分類器です。点数は、バイアスに、判定の各素性の重みと、素性の各対の重みを足したものです。CaboCha・J.DepP の 2 次の多項式カーネルを展開すると、この形になります。重みはメモリ上のハッシュ表で持ちます。手や実験で作った重みで解析するには、これで足ります。

## 素性

1 回の判定の素性は文字列で、名前は CaboCha と同じです。ほとんどの素性は文節ごとに 1 回作り、判定のときに、見ている文節の分を組み合わせます。素性は並べ替えてあり、同じものは 1 つだけです。

| 群 | 素性 | 例 |
| --- | --- | --- |
| `distance` | 係り元から係り先の候補までの文節の数。1、2〜5、6 以上 | `DIST:1`、`DIST:2-5`、`DIST:6-` |
| `source` | 係り元の文節。主辞（`H`）、機能語（`F`）、最初の形態素（`L`）、最後の形態素（`R`）の表層、品詞の各階層（最初の `*` まで）、活用形。句読点と括弧。文節の中のすべての助詞。文の最初の文節か、最後の文節か | `SHS:彼`、`SHP0:名詞`、`SFS:は`、`SRF:基本形`、`SPUNC:、`、`SOB:「`、`SOB:1`、`SCASE:は`、`SBOS:1` |
| `target` | 係り先の候補について同じもの。先頭の字は `D` | `DHS:読ん`、`DHF:連用タ接続`、`DEOS:1` |
| `left` | 係り元の 1 つ前の文節の、機能語の表層 | `LF:は` |
| `right` | 係り先の候補の 1 つ後の文節の、最初の形態素・主辞・機能語の表層 | `RL:言う`、`RH:言う`、`RF:言う` |
| `children` | すでに係り元に係った文節（`a:`）と、係り先の候補に係った文節（`A:`）。それぞれを機能語で表す。助詞・副詞・連体詞・接続詞なら表層、そうでなければ活用形、それも無ければ品詞 | `a:は`、`A:を`、`A:基本形`、`a:名詞-一般` |
| `gap` | 係り元と係り先の候補の間の文節。句読点、括弧、機能語が助詞のときのその助詞。間で括弧が開くか閉じるか | `GPUNC:、`、`GOB:「`、`GCASE:を`、`GNB:1`（どちらもない）、`GOB:1`、`GCB:1`、`GBB:1`（両方） |

それまでの判定に左右されるのは、`children` の素性だけです。

解析器のコードには、品詞の名前も列の番号も書いていません。主辞と機能語、品詞の階層、活用形、助詞（`Schema::particles`）、係った文節の素性に表層を使う語（`Schema::lexicalized`）、記号は、すべてスキーマから取ります。

### 素性の集合

どの群を作るかは、文節の区切りのテンプレートと同じく、データです。`FeatureSet` は、群の名前をコンマでつないだ文字列で書きます。`FeatureSet::cabocha()` は全部の群を持ちます。

```text
distance,source,target,left,right,children,gap
```

分類器は、1 つの集合の素性で学習し、同じ集合で解析します。

### CaboCha との違い

| | CaboCha | このクレート | 理由 |
| --- | --- | --- | --- |
| 表層 | 表で正規化する（全角と半角） | トークナイザーが出したまま | スキーマが、記号を品詞と表層の両方で、全角も半角も見分ける |
| 句読点と括弧 | 。と 、、開き括弧 10 種と閉じ括弧 10 種を、正規化した表層で見分ける | スキーマのもの。`,` `.` `，` `．` と、より多くの括弧も含む | IPADIC は、半角の記号を名詞として出す |
| 主辞と機能語 | タグセットごとのパターンによる `findHead` | `Schema::positions`。IPADIC のパターンを足してある | [主辞と機能語](./head_and_function_word.md)を参照 |
| 係り先が分からない文節 | 次の文節に係るとして進め、事例にしない（部分的な学習） | 扱わない。係り先の無い文節は根 | 学習データは、すべての文節の係り先を持つ |
| 最後の文節への係りの点数 | 直前の判定の点数（関係のない値） | 0 | その判定は尋ねない |

J.DepP には、CaboCha に無い素性があります。2 つの文節の主辞の表層が同じかどうか（並列の手がかり）と、括弧の有無ではなく釣り合いです。これらは、[#1437](https://github.com/lindera/lindera/issues/1437) で素性の集合を比べるときの候補です。

## 学習事例

分類器は、解析器が尋ねる判定で学習します。`parser::instances` は、係り先が分かっている文でループを回し、分類器の代わりに正解に答えさせます。CaboCha の学習モードと同じです。1 回の判定が 1 件の事例で、その素性と、係り元が係り先の候補に係るかどうかを持ちます。`children` の素性は、その時点までの正解のものです。

```rust
use lindera_dependency::lattice;
use lindera_dependency::parser::{self, FeatureSet};
use lindera_dependency::Schema;

// 彼は | 本を | 読んだと | 言う, with 彼は depending on 言う.
let text = "\
* 0 3D
彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 3D
読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
と\t助詞,格助詞,引用,*,*,*,と,ト,ト
* 3 -1D
言う\t動詞,自立,*,*,五段・ワ行促音便,基本形,言う,イウ,イウ
EOS
";
let sentence = &lattice::parse(text)?[0];
let set = FeatureSet::parse("distance,left,children")?;
for instance in parser::instances(&Schema::ipadic(), &set, sentence)? {
    let label = if instance.label { "+1" } else { "-1" };
    println!("{label} {}", instance.features.join(" "));
}
```

```text
-1 DIST:1
+1 DIST:1 LF:は
-1 A:を DIST:2-5
```

上の表の 3 回の判定です。3 回目では、本を がすでに係り先の候補の 読んだと に係っているので、`A:を` が付きます。

解析器が作れない木の文（係り先が左にある、係り受けが交差する、根が最後の文節でない）は、エラーになります。[変換](./training_data.md)は、そういう文を学習データに入れません。

### コマンド

`train` feature を付けると、`lindera-dependency instances` が、ラティス形式のファイルの学習事例を、IPADIC のスキーマで書き出します。

```shell
lindera-dependency instances --input train.lattice --output train.instances
```

1 行が 1 件の事例です。`+1` か `-1` の後に、素性の文字列がタブ区切りで続きます。文ごとに空行が 1 つ入るので、ファイルを文の境目で分割できます。`--features` は素性の集合を受け取り、既定は CaboCha の集合です。

報告は標準エラーに出ます。UD Japanese-GSD の train では、次のとおりです。

```text
train.lattice
features: distance,source,target,left,right,children,gap
sentences: 7024
bunsetsu: 56396
instances: 66000
positive: 33512
negative: 32488
```

コマンドは、正解に答えさせても木が戻らない文で止まります。`scripts/dependency/parser_instances.sh` は、学習データの 4 つのファイルにこのコマンドを流します。

| ファイル | 文 | 文節 | 事例 | 正 | 負 |
| --- | --- | --- | --- | --- | --- |
| UD Japanese-GSD train | 7,024 | 56,396 | 66,000 | 33,512 | 32,488 |
| UD Japanese-GSD dev | 506 | 4,147 | 4,881 | 2,474 | 2,407 |
| UD Japanese-GSD test | 542 | 4,509 | 5,377 | 2,707 | 2,670 |
| UD Japanese-PUD | 994 | 9,843 | 12,739 | 6,077 | 6,662 |

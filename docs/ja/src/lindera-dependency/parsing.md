# 係り受けの解析

係り受けの解析は、文の文節ごとに、その文節が係る先の文節を求めます。`parser` モジュールは、CaboCha・J.DepP と同じ方法で求めます。スタックを使って文節を左から右へたどり（shift-reduce）、分類器が 1 回に 1 つの問いに答えます。

Lindera は [IPADIC のモデル](./model_file.md#ipadic-のモデル)を公開しています。自分のモデルを作るときは、[学習データ](./training_data.md)から[学習](#モデルを学習する)します。数秒で終わります。

## モデルで解析する

[モデルファイル](./model_file.md)は、1 つの辞書の文節の区切りと解析器を持ちます。両方があれば、文を形態素から係り受けまで解析できます。

```rust
use std::io;

use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::Model;
use lindera_dependency::Sentence;

let model = Model::new(std::fs::read("ipadic.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");
let parser = model.parser().expect("the file has a parser");

// The morphemes of 彼は赤い花を見た。 as IPADIC gives them.
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("赤い", "形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ"),
    LatticeMorpheme::new("花", "名詞,一般,*,*,*,*,花,ハナ,ハナ"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("見", "動詞,自立,*,*,一段,連用形,見る,ミ,ミ"),
    LatticeMorpheme::new("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];
let ranges = chunker.chunk(&morphemes)?;
let bunsetsu = parser.parse(&morphemes, &ranges)?;

// The result is a sentence the lattice format can hold.
let sentence = Sentence::new(morphemes, bunsetsu)?;
lattice::write(&mut io::stdout(), &sentence)?;
```

<!-- markdownlint-disable MD010 -->
```text
* 0 3D 0/1 0.000000
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/0 0.704616
赤い	形容詞,自立,*,*,形容詞・アウオ段,基本形,赤い,アカイ,アカイ
* 2 3D 0/1 0.000000
花	名詞,一般,*,*,*,*,花,ハナ,ハナ
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 3 -1D 0/1 0.000000
見	動詞,自立,*,*,一段,連用形,見る,ミ,ミ
た	助動詞,*,*,*,特殊・タ,基本形,た,タ,タ
。	記号,句点,*,*,*,*,。,。,。
EOS
```
<!-- markdownlint-enable MD010 -->

各文節は、係り先、主辞と機能語の位置、その文節を係らせた判定の点数を持ちます。点数は 0 より大きく、大きいほど確かです。ここでは、赤い が 花を に、点数 0.70 で係っています。根と、根に係る文節の点数は 0 です。その判定は尋ねないためです。`Model::analyze` は、この 2 段を 1 回の呼び出しで行います。多くの文を解析するときは、上のように区切りと解析器を 1 回だけ取り出してください。どちらも `&self` を取り、複数のスレッドで使えます。

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

モデルファイルの解析器は、ファイルの分類器を持つ `Parser` です。`Parser` は部品からも作れます。辞書の[スキーマ](./head_and_function_word.md)、素性の集合、分類器の 3 つです。次の分類器は手で作ったもので、重みは 1 つだけです。を で終わる文節は、次の文節に係ります。

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

このクレートの分類器はどれも、バイアスに、判定の各素性の重みと、素性の各対の重みを足して点数にします。CaboCha・J.DepP の 2 次の多項式カーネルを展開すると、この形になります。

| 分類器 | 重みの置き場所 |
| --- | --- |
| `ParserPart` | モデルファイルのバイト列の上。`Model::parser` が、これを持つ解析器を返す |
| `ParserModel` | メモリ上。学習の結果で、`ModelBuilder` が書き出すもの |
| `LinearClassifier` | メモリ上の、素性の名前で引くハッシュ表。手で作った重みのためのもの |

学習したモデルは、対の重みを、決まった大きさの表に持ちます。対はこの表にハッシュで割り当てるので、同じ場所に当たった対は重みを共有します。対ごとに重みを持つ形も選べます（`--pairs exact`）。[既定の選び方](#既定の選び方)を参照してください。

## 素性

1 回の判定の素性は文字列で、名前は CaboCha と同じです。CaboCha に無い群が 1 つあります。ほとんどの素性は文節ごとに 1 回作り、判定のときに、見ている文節の分を組み合わせます。素性は並べ替えてあり、同じものは 1 つだけです。

| 群 | 素性 | 例 |
| --- | --- | --- |
| `distance` | 係り元から係り先の候補までの文節の数。1、2〜5、6 以上 | `DIST:1`、`DIST:2-5`、`DIST:6-` |
| `source` | 係り元の文節。主辞（`H`）、機能語（`F`）、最初の形態素（`L`）、最後の形態素（`R`）の表層、品詞の各階層（最初の `*` まで）、活用形。句読点と括弧。文節の中のすべての助詞。文の最初の文節か、最後の文節か | `SHS:彼`、`SHP0:名詞`、`SFS:は`、`SRF:基本形`、`SPUNC:、`、`SOB:「`、`SOB:1`、`SCASE:は`、`SBOS:1` |
| `target` | 係り先の候補について同じもの。先頭の字は `D` | `DHS:読ん`、`DHF:連用タ接続`、`DEOS:1` |
| `left` | 係り元の 1 つ前の文節の、機能語の表層 | `LF:は` |
| `right` | 係り先の候補の 1 つ後の文節の、最初の形態素・主辞・機能語の表層 | `RL:言う`、`RH:言う`、`RF:言う` |
| `children` | すでに係り元に係った文節（`a:`）と、係り先の候補に係った文節（`A:`）。それぞれを機能語で表す。助詞・副詞・連体詞・接続詞なら表層、そうでなければ活用形、それも無ければ品詞 | `a:は`、`A:を`、`A:基本形`、`a:名詞-一般` |
| `gap` | 係り元と係り先の候補の間の文節。句読点、括弧、機能語が助詞のときのその助詞。間で括弧が開くか閉じるか | `GPUNC:、`、`GOB:「`、`GCASE:を`、`GNB:1`（どちらもない）、`GOB:1`、`GCB:1`、`GBB:1`（両方） |
| `after` | 係り元と係り先の候補の、主辞の直後の形態素（`N`）。表層、品詞の各階層、活用形 | `SNS:し`、`SNP0:動詞`、`SNF:連用形`、`DNS:こと` |

それまでの判定に左右されるのは、`children` の素性だけです。

`after` の群は、CaboCha のものではありません。CaboCha は、文節の中で機能語でない最後の語を主辞にします。[スキーマ](./head_and_function_word.md)の主辞は内容語で、その 1 つ前の語になることがあります。利用 / し / た の 利用、読む / こと / を の 読む です。このとき、文節が全体として動詞なのか名詞なのかは、主辞の直後の語で分かります。解析器にはこの情報が要ります。この素性を足すと、主辞を内容語のままにして、CaboCha の主辞を使ったときと同じ精度になります。

解析器のコードには、品詞の名前も列の番号も書いていません。主辞と機能語、品詞の階層、活用形、助詞（`Schema::particles`）、係った文節の素性に表層を使う語（`Schema::lexicalized`）、記号は、すべてスキーマから取ります。

### 素性の集合

どの群を作るかは、文節の区切りのテンプレートと同じく、データです。`FeatureSet` は、群の名前をコンマでつないだ文字列で書きます。既定の `FeatureSet::default()` は全部の群を持ちます。

```text
distance,source,target,left,right,children,gap,after
```

`FeatureSet::cabocha()` は、ここから `after` を除いたものです。モデルファイルは、解析器の素性の集合を持ちます。

分類器は、1 つの集合の素性で学習し、同じ集合で解析します。

### CaboCha との違い

| | CaboCha | このクレート | 理由 |
| --- | --- | --- | --- |
| 表層 | 表で正規化する（全角と半角） | トークナイザーが出したまま | スキーマが、記号を品詞と表層の両方で、全角も半角も見分ける |
| 句読点と括弧 | 。と 、、開き括弧 10 種と閉じ括弧 10 種を、正規化した表層で見分ける | スキーマのもの。`,` `.` `，` `．` と、より多くの括弧も含む | IPADIC は、半角の記号を名詞として出す |
| 主辞と機能語 | タグセットごとのパターンによる `findHead` | `Schema::positions`。IPADIC のパターンを足してある | [主辞と機能語](./head_and_function_word.md)を参照 |
| 主辞の直後の形態素 | そのような素性は無い | `after` の群 | スキーマの主辞は内容語であるため |
| 係り先が分からない文節 | 次の文節に係るとして進め、事例にしない（部分的な学習） | 扱わない。係り先の無い文節は根 | 学習データは、すべての文節の係り先を持つ |
| 最後の文節への係りの点数 | 直前の判定の点数（関係のない値） | 0 | その判定は尋ねない |

J.DepP には、CaboCha に無い素性があります。2 つの文節の主辞の表層が同じかどうか（並列の手がかり）と、括弧の有無ではなく釣り合いです。これらは入れていません。精度の目標に、これら無しで届いたためです。

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

`train` feature を付けると、`lindera-dependency instances` が、ラティス形式のファイルの学習事例を、IPADIC のスキーマで書き出します。`--dictionary-name` で辞書を指定すると、その辞書のスキーマを使います。

```shell
lindera-dependency instances --input train.lattice --output train.instances
```

1 行が 1 件の事例です。`+1` か `-1` の後に、素性の文字列がタブ区切りで続きます。文ごとに空行が 1 つ入るので、ファイルを文の境目で分割できます。`--features` は素性の集合を受け取り、既定は全部の群です。

報告は標準エラーに出ます。UD Japanese-GSD の train では、次のとおりです。

```text
train.lattice
features: distance,source,target,left,right,children,gap,after
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

## モデルを学習する

学習は `train` feature の裏にあります。`train-parser` は、[学習データ](./training_data.md)から IPADIC のスキーマで解析器を学習し、[モデルファイル](./model_file.md)を書き出します。`--dictionary-name` で辞書を指定すると、その辞書のスキーマを使います。`--chunker` を付けると、ほかのモデルファイルの区切りを出力に入れます。これで、その辞書のモデルの全体になります。そのファイルは、同じ辞書のものでなければなりません。

```shell
cargo run --release -p lindera-dependency --features train -- train-chunker \
    --input train.lattice --output chunker.model
cargo run --release -p lindera-dependency --features train -- train-parser \
    --input train.lattice --chunker chunker.model --output ipadic.model
```

IPADIC で変換した UD Japanese-GSD の train では、`train-parser` は約 3 秒で終わり、最後に次を出力します。

```text
train.lattice: 7024 sentences
features: distance,source,target,left,right,children,gap,after
instances: 66000
passes: 19
updates: 21374
feature strings: 62380
pair weights: 2762034
bytes: 6258548
```

この後に、学習の時間を書いた `seconds:` の行が続きます（ここでは省いています）。同じデータで学習し直すと、同じバイト列になります。

| オプション | 既定 | 意味 |
| --- | --- | --- |
| `--chunker` | なし | 区切りを出力に入れる、元のモデルファイル |
| `--algorithm` | `dcd` | `dcd` は、サポートベクターマシンの dual coordinate descent。CaboCha の学習の方法。`pa` は passive-aggressive。J.DepP の学習の方法 |
| `--cost` | `dcd` は `0.0015`、`pa` は `0.001` | 学習の 1 件の判定が重みに足せる上限 |
| `--iterations` | `10` | `pa` のとき、学習の判定を回る回数 |
| `--averaged` | オフ | `pa` のとき、判定ごとの重みの平均を使う |
| `--features` | 全部の群 | [素性の集合](#素性の集合) |
| `--pairs` | `hashed` | `hashed` は、対が共有する決まった大きさの表。`exact` は、対ごとの重み |
| `--bits` | `22` | `hashed` のとき、表は 2^bits 個の重みを持つ |
| `--min-support` | `1` | `exact` のとき、対が重みを持つのに要る、学習の判定の数 |
| `--pair-threshold` | `0` | `exact` のとき、重みがこれより 0 に近い対を除く |
| `--min-count` | `1` | 素性を使うのに要る、学習の判定の数 |
| `--quantization` | `i8` | ファイルの中の重み。`f32` はそのまま、`i16` と `i8` は整数と尺度の積 |
| `--seed` | `1` | `dcd` のとき、判定を回る順の種 |

[`parse`](./command_line.md) に `--input-layer` を付けると、モデルで、ラティス形式のファイルの係り先を求めます。[`evaluate`](./evaluation.md) で正解と比べるためのものです。

```shell
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer bunsetsu --model ipadic.model --input dev.lattice --output output.lattice
cargo run --release -p lindera-dependency --features train -- parse \
    --input-layer morphemes --model ipadic.model --input dev.lattice --output output.lattice
```

`bunsetsu` では、入力の文節を解析します。解析器だけを測る形です。`morphemes` では、入力の文節は読みません。形態素をモデルの区切りで文節に区切ってから解析します。実際に使うときの解析です。`--input-layer` が無ければ、入力はテキストです。[コマンドライン](./command_line.md)を見てください。

## 精度

UD Japanese-GSD（commit c662dc9）の train と IPADIC で、既定の設定で学習した値です。係り受けの精度は、各文の最後を除く文節で数えます（[level 1](./evaluation.md)）。

| | 正解の文節で | 形態素から | 区切りの F |
| --- | --- | --- | --- |
| UD Japanese-GSD dev | 90.11% (3,281 / 3,641) | 85.39% (3,109 / 3,641) | 96.85 |
| UD Japanese-GSD test | 88.53% (3,512 / 3,967) | 84.47% (3,351 / 3,967) | 96.92 |
| UD Japanese-PUD | 88.81% (7,859 / 8,849) | 86.19% (7,627 / 8,849) | 98.04 |

このプロジェクトの目標は、同じデータで学習した CaboCha・J.DepP の値から決めたもので、正解の文節で dev 89.0・test 87.8、形態素から dev 85.0・test 84.3 でした。この基準は、今のリポジトリにあるものでは測り直せません（[#1433](https://github.com/lindera/lindera/issues/1433)）。比べている相手は記録に残る数字で、2 つのツールを動かした結果ではありません。

区切りと解析器を入れたモデルファイルは 6,258,548 バイトです。AMD Ryzen AI Max+ 395 の 1 コアで、読み込みは約 3 ms、区切りと解析を合わせて毎秒 約 8,700 文です。[モデルファイル](./model_file.md#大きさと速さ)を参照してください。

## 既定の選び方

train の 10 分割交差検証を、正解の文節の上で行って選びました。有意な差の無い設定（係り受けごとの McNemar の正確検定）は同じとみなし、小さいほう・速いほうを選びます。交差検証で差の付かない大きさは、dev で選びます。

| 選んだもの | 10 分割の level 1 | 分かったこと |
| --- | --- | --- |
| dual coordinate descent、コスト 0.0015 | 88.96% | 最良のコスト。0.003 は 88.91%（p = 0.47）、0.001 は 88.79%（p < 0.001） |
| passive-aggressive、平均化、コスト 0.0005、30 回 | 88.88% | dual coordinate descent と差が無く（p = 0.22）、時間も同じ。回る回数を選ぶ必要があり、dual coordinate descent は自分で止まる |
| CaboCha の集合から `right` を除く | 88.53% | 最も効く群（p < 0.001）。`gap` を除くと 88.75%、`children` を除くと 88.87% |
| `after` の群を足す | 89.19% | CaboCha の集合より良い（p < 0.001） |
| 対ごとの重み。38 MB の表 | 89.19% | 毎秒 2,700 文 |
| 対を 2^22 個の重みにハッシュで割り当てる | 89.21% | 差が無く（p = 0.60）、毎秒 16,700 文。2^18 から 2^21 も差が無く、その中で dev が最良なのが 2^22 |
| 1 バイトの重み | 89.21% | 4 バイトと差が無い（p = 0.86）。2 バイトの重みは出力が同じ |

全部の設定の表は[クレートの README](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#how-the-defaults-of-the-parser-were-chosen)にあり、[`scripts/dependency/parser_experiments.sh`](https://github.com/lindera/lindera/tree/main/scripts/dependency) で流し直せます。

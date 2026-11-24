# 文節の区切り

文節の区切り（チャンキング）は、文の形態素を文節にまとめます。形態素ごとに、文節を始める（`B`）か、続ける（`I`）かを決めます。CaboCha と同じく、この 2 つのラベルの条件付き確率場を使います。

区切りは既定のビルドに入っていて、ほかのクレートを必要としません。モデルは、素性の文字列から `B` と `I` の重みへの表と、遷移の重みで、ラベルは Viterbi で求めます。表は、[モデルファイル](./model_file.md)の中の位置のまま読みます。学習は `train` feature の裏にあります。

## 文を区切る

```rust
use lindera_dependency::lattice::{self, LatticeMorpheme};
use lindera_dependency::model::Model;
use lindera_dependency::{Bunsetsu, Morpheme, Sentence};

let model = Model::new(std::fs::read("chunker.model")?)?;
let chunker = model.chunker().expect("the file has a chunker");

// The morphemes of 彼は本を読んだ。 as IPADIC gives them. Any type that
// implements `Morpheme` will do.
let morphemes = vec![
    LatticeMorpheme::new("彼", "名詞,代名詞,一般,*,*,*,彼,カレ,カレ"),
    LatticeMorpheme::new("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
    LatticeMorpheme::new("本", "名詞,一般,*,*,*,*,本,ホン,ホン"),
    LatticeMorpheme::new("を", "助詞,格助詞,一般,*,*,*,を,ヲ,ヲ"),
    LatticeMorpheme::new("読ん", "動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン"),
    LatticeMorpheme::new("だ", "助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ"),
    LatticeMorpheme::new("。", "記号,句点,*,*,*,*,。,。,。"),
];

// The ranges of the bunsetsu, as indices into the morphemes.
let ranges = chunker.chunk(&morphemes)?;
assert_eq!(ranges, [0..2, 2..4, 4..7]);
```

`Chunker::chunk` は `&self` を取り、chunker は `Send + Sync` なので、1 つの chunker を複数のスレッドで使えます。先頭の形態素は必ず文節を始め、空の文には文節がありません。

形態素は、モデルを学習した辞書のもので、同じ設定で分かち書きしたものでなければなりません。モデルは 1 つの辞書に属します。モデルのスキーマが読む列より少ない列しか持たない形態素は、エラーで断ります。[辞書の照合](./model_file.md#辞書の照合)を参照してください。

### 主辞と機能語を付ける

モデルファイルは、辞書の[スキーマ](./head_and_function_word.md)を持っていて、そこから各文節の主辞と機能語が決まります。次の例は、結果をラティス形式で書き出します。解析器がまだ無いので、ここでの係り先は仮のものです。どの文節も次の文節に係るとしています。

```rust
let schema = model.schema();
let count = ranges.len();
let bunsetsu = ranges
    .into_iter()
    .enumerate()
    .map(|(index, range)| {
        let positions = schema.positions(&morphemes[range.clone()]);
        let mut bunsetsu = Bunsetsu::new(range, (index + 1 < count).then_some(index + 1));
        bunsetsu.positions = positions;
        bunsetsu
    })
    .collect();
let sentence = Sentence::new(morphemes, bunsetsu)?;
let mut output = Vec::new();
lattice::write(&mut output, &sentence)?;
print!("{}", String::from_utf8(output)?);
```

```text
* 0 1D 0/1
彼	名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は	助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1
本	名詞,一般,*,*,*,*,本,ホン,ホン
を	助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 -1D 0/1
読ん	動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ	助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。	記号,句点,*,*,*,*,。,。,。
EOS
```

## モデルを学習する

モデルはまだ配布していません。[学習データ](./training_data.md)から学習します。

```shell
cargo run --release -p lindera-dependency --features train -- train-chunker \
    --input train.lattice --output chunker.model
```

IPADIC で変換した UD Japanese-GSD の train では、数秒で終わり、最後に次を出力します。

```text
train.lattice: 7024 sentences
iterations: 126
feature strings: 4325
weights: 8656
bytes: 261851
```

出力は、辞書 `ipadic` の[モデルファイル](./model_file.md)で、部分は区切りだけです。同じデータで学習し直すと、同じバイト列になります。重みは `f64` です。形式は、最初のリリースまでに変わることがあります。

| オプション | 既定 | 意味 |
| --- | --- | --- |
| `--regularization` | `l1` | `l1` か `l2` |
| `--lambda` | `0.5` | 正則化の係数 |
| `--columns` | `surface,pos,cform` | テンプレートが読む列。`surface`（表層）、`pos`（品詞の階層を `-` でつないだもの）、`ctype`（活用型）、`cform`（活用形）、`base`（原形） |
| `--template` | CaboCha の区切りのテンプレートに、3 列目の 5 行を足したもの | CRF++ の形式のファイル |
| `--max-iterations` | `1000` | 反復の上限 |
| `--threads` | CPU の数 | スレッドの数 |

`chunk` は、モデルでファイルの形態素を区切り直します。[`evaluate`](./evaluation.md) で正解と比べるためのものです。

```shell
cargo run --release -p lindera-dependency --features train -- chunk \
    --model chunker.model --input dev.lattice --output output.lattice
```

[解析器](./parsing.md)の学習済みのモデルがまだ無いので、どの文節も次の文節に係るとして書き出します。

## 精度

既定の設定で、UD Japanese-GSD（commit c662dc9）の train を IPADIC で学習したときの値です。

| | F | 精度 | 再現率 | すべての文節が合った文 |
| --- | --- | --- | --- | --- |
| UD Japanese-GSD dev | 96.85 | 97.02% (4,009 / 4,132) | 96.67% (4,009 / 4,147) | 432 / 506 |
| UD Japanese-GSD test | 96.92 | 96.85% (4,373 / 4,515) | 96.98% (4,373 / 4,509) | 459 / 542 |
| UD Japanese-PUD | 98.04 | 98.05% (9,649 / 9,841) | 98.03% (9,649 / 9,843) | 885 / 994 |

既定の設定は、train の 10 分割の交差検証で選びました。活用形は効き、活用型は効かず、L1 は L2 より良く、モデルは約 50 分の 1 になります。交差検証の表は[クレートの README](https://github.com/lindera/lindera/blob/main/lindera-dependency/README.md#how-the-defaults-were-chosen) にあり、[`scripts/dependency`](https://github.com/lindera/lindera/tree/main/scripts/dependency) で流し直せます。

## 素性の作り方

素性の文字列は 2 段階で作り、どちらも差し替えられます。

1. **列。** 形態素ごとに、モデルの列をスキーマを通して読みます。表層、品詞、活用形です。
2. **テンプレート。** CRF++ の形式のテンプレートが、その形態素と前後の形態素の列を、素性の文字列に展開します。`U02:%x[0,0]` はその形態素の 1 列目、`U11:%x[-1,1]` は 1 つ前の形態素の 2 列目です。文の外の位置は `_B-1`、`_B+1` などになります。

復号は、素性の文字列と重みの表だけを見ます。`Chunker::features` が文字列を作り、`Chunker::labels` がそれを復号するので、自分で書いた取り出し方から同じ復号を使えます。

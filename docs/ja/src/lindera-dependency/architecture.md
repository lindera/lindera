# アーキテクチャ

## モジュール構成

```text
lindera-dependency/src/
├── lib.rs                # Public API re-exports
├── morpheme.rs           # Morpheme trait, DictionaryMorpheme
├── sentence.rs           # Sentence, Bunsetsu, ArcType, Violation
├── lattice.rs            # Reading and writing the lattice format of CaboCha
├── schema.rs             # Schema: columns, head word and function word
├── schema/
│   ├── pattern.rs        # Pattern over the morphemes of a bunsetsu
│   └── symbols.rs        # Marks, Symbols: punctuation and brackets
├── chunker.rs            # Chunker
├── chunker/
│   ├── columns.rs        # Column: what a template reads from a morpheme
│   ├── template.rs       # Template in the format of CRF++
│   ├── model.rs          # ChunkerData, ChunkerModel: the table of weights
│   ├── decode.rs         # Viterbi over the two labels
│   └── train.rs          # Training with lindera-crf (train)
├── model.rs              # Model, ModelBuilder: the model file of a dictionary
├── model/
│   └── table.rs          # A table from strings to values, read in place
├── parser.rs             # Parser
├── parser/
│   ├── features.rs       # Group, FeatureSet: the features of a decision
│   ├── shift_reduce.rs   # The loop over the bunsetsu
│   ├── classifier.rs     # Classifier, LinearClassifier
│   ├── model.rs          # ParserModel, ParserPart: the weights, in memory and on a file
│   ├── train.rs          # Training: dual coordinate descent, passive-aggressive (train)
│   └── oracle.rs         # Training instances from an answer
├── corpus.rs             # Training data from a dependency corpus (train)
├── corpus/
│   ├── conllu.rs         # Reading CoNLL-U
│   ├── convert.rs        # Word dependencies to bunsetsu dependencies
│   └── project.rs        # Bunsetsu onto the morphemes of a dictionary
├── evaluation.rs         # Chunking and dependency accuracy (train)
├── error.rs              # Error types
├── bytes.rs              # Helpers for the bytes of a model
└── bin/
    ├── lindera-dependency.rs   # The command: parse, chunk (cli)
    └── lindera-dependency/
        └── train.rs            # The subcommands of training and evaluation (train)
```

## 主要コンポーネント

### Morpheme

形態素は、表層と、辞書の素性の列です。`Morpheme` は trait なので、このクレートはトークナイザに依存せず、自分で定義した型も解析できます。

```rust
use lindera_dependency::Morpheme;

struct Word {
    surface: &'static str,
    features: [&'static str; 2],
}

impl Morpheme for Word {
    fn surface(&self) -> &str {
        self.surface
    }

    fn feature(&self, index: usize) -> Option<&str> {
        self.features.get(index).copied()
    }
}

let word = Word { surface: "本", features: ["名詞", "一般"] };
assert_eq!(word.feature(1), Some("一般"));
assert_eq!(word.features(), "名詞,一般");
```

列の意味は辞書で決まります。IPADIC では、列 0 が品詞、列 6 が原形です。このクレートは、列を番号ではなく、[スキーマ](./head_and_function_word.md)を通して役割で読みます。

### Sentence / Bunsetsu

`Sentence` は、形態素と、それをまとめる `Bunsetsu`（文節）を持ちます。文節は形態素の範囲で、次を持ちます。

- 係り先。係る先の文節の番号で、根は `None`
- 係りの種類（`ArcType`）。`D` はふつうの係り受け、`P` は並列、`A` は同格、`I` は不完全な並列
- 主辞と機能語の位置と、スコア（どちらも省略できる）

`Sentence::violations` は、文が、解析器の仮定する木でない点を報告します。根は 1 つで最後の文節、係り先はすべて右、係りは交差しない、という木です。

### ラティス形式

`lattice` モジュールは、`cabocha -f1` の形式を読み書きします。CaboCha と J.DepP のツールがやり取りする形式です。文節の行は `*` で始まり、文節の番号、係り先の番号と係りの種類（根は `-1`）、省略できる主辞と機能語の位置、スコアが続きます。形態素の行は、表層、タブ、素性の文字列です。`EOS` が文の終わりです。

```rust
use lindera_dependency::Morpheme;
use lindera_dependency::lattice;

// 彼は | 本を | 読んだ。 as `cabocha -f1` writes it.
let text = "\
* 0 2D 0/1 -1.500000
彼\t名詞,代名詞,一般,*,*,*,彼,カレ,カレ
は\t助詞,係助詞,*,*,*,*,は,ハ,ワ
* 1 2D 0/1 -1.500000
本\t名詞,一般,*,*,*,*,本,ホン,ホン
を\t助詞,格助詞,一般,*,*,*,を,ヲ,ヲ
* 2 -1D 0/1 0.000000
読ん\t動詞,自立,*,*,五段・マ行,連用タ接続,読む,ヨン,ヨン
だ\t助動詞,*,*,*,特殊・タ,基本形,だ,ダ,ダ
。\t記号,句点,*,*,*,*,。,。,。
EOS
";
let sentences = lattice::parse(text)?;
let sentence = &sentences[0];
for bunsetsu in &sentence.bunsetsu {
    let surface: String = sentence.morphemes[bunsetsu.morphemes.clone()]
        .iter()
        .map(|morpheme| morpheme.surface())
        .collect();
    println!("{surface} -> {:?} ({})", bunsetsu.head, bunsetsu.arc_type);
}
assert!(sentence.violations().is_empty());

// Writing gives the text back.
let mut output = Vec::new();
lattice::write(&mut output, sentence)?;
assert_eq!(output, text.as_bytes());
```

```text
彼は -> Some(2) (D)
本を -> Some(2) (D)
読んだ。 -> None (D)
```

文節の行がない文は、形態素だけの文です。トークナイザが MeCab の形式で書き出すものと同じです。スコアは、CaboCha と同じく小数 6 桁で書き出します。

### Schema

`Schema` は、辞書について分かっていることをデータとして持ちます。どの列が品詞・活用・原形か、どの形態素が機能語か、どれが助詞か、記号をどう見分けるか、です。[主辞と機能語](./head_and_function_word.md)を参照してください。

### Chunker / ChunkerModel

`Chunker` は、素性の文字列から重みへの表と、その表の学習に使ったスキーマ、列、テンプレートで、形態素を文節に区切ります。これらは `ChunkerData` トレイトを通して読みます。読む先は、メモリ上の `ChunkerModel`（学習の結果）か、モデルファイルの区切りの部分です。[文節の区切り](./chunking.md)を参照してください。

### Model / ModelBuilder

`Model` は、辞書のモデルファイルです。所有するか借りるバイト列を、その場で読みます。辞書の名前、スキーマ、学習した部分（文節の区切りと解析器）を持ちます。`Model::analyze` は、文を形態素から係り受けまで解析します。`ModelBuilder` はファイルを書きます。[モデルファイル](./model_file.md)を参照してください。

### Parser / Classifier

`Parser` は、shift-reduce で各文節の係り先を求めます。1 回の判定の素性は、スキーマを通して作る文字列です。`FeatureSet` がどの群の素性を作るかを決め、`Classifier` が、素性の重みとその対の重みを足して点数を付けます。モデルファイルの分類器は `ParserPart` で、ファイルのバイト列の上で重みを読みます。`ParserModel` は、学習の結果の重みをメモリに持ちます。`LinearClassifier` は、手で作った重みを受け取ります。`parser::instances` は正解のある文から学習事例を作り、`parser::train` がそれで学習します。[係り受けの解析](./parsing.md)を参照してください。

### コマンド

`cli` feature を付けると、`lindera-dependency` コマンドがビルドされます。分かち書きには `lindera-segmenter` を使い、学習のコードは入りません。`train` feature を付けると、学習データを作る、学習する、評価するサブコマンドが加わります。

| サブコマンド | Feature | 内容 | ページ |
| --- | --- | --- | --- |
| `parse` | `cli` | テキストか、分かち書き済み・文節つきの入力を解析し、ラティス形式で書く | [コマンドライン](./command_line.md) |
| `chunk` | `cli` | モデルファイルで、ファイルの形態素を区切り直す | [文節の区切り](./chunking.md) |
| `convert` | `train` | 文節のラベルを持つ CoNLL-U のファイルから、学習データを作る | [学習データ](./training_data.md) |
| `evaluate` | `train` | 出力を正解と比べる | [評価](./evaluation.md) |
| `split` | `train` | 交差検証の分割を書き出す | [評価](./evaluation.md) |
| `train-chunker` | `train` | 文節の区切りを学習し、モデルファイルを書く | [文節の区切り](./chunking.md) |
| `instances` | `train` | 解析器の学習事例を書き出す | [係り受けの解析](./parsing.md) |
| `train-parser` | `train` | 解析器を学習し、モデルファイルを書く。ほかのファイルの区切りを入れられる | [係り受けの解析](./parsing.md) |

コマンドは `lindera-cli` ではなく、このクレートにあります。ラティス形式を書き出し、学習を持つコマンドで、トークナイザーを使うだけの人には要らないためです。`lindera-cli` は、解析器を[トークンフィルター](./token_filter.md)として動かします。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| （なし） | 型、ラティス形式、スキーマ、モデルファイル、文節の区切り、解析器。依存がなく、`wasm32-unknown-unknown` 向けにビルドできる | Yes |
| `cli` | `lindera-dependency` コマンドの `parse` と `chunk`。分かち書きは `lindera-segmenter`。学習のコードは入らない | No |
| `train` | `corpus` と `evaluation` のモジュール、`lindera-crf` による区切りの学習と解析器の学習、そのサブコマンド（`cli` を含む） | No |
| `embed-ipadic` | コマンドに IPADIC を埋め込む。`--dict embedded://ipadic` で使う（`cli` を含む） | No |
| `embed-model-ipadic` | [IPADIC のモデル](./model_file.md#埋め込む)を埋め込む。`embedded::model("ipadic")` と `--model embedded://ipadic` で使う。ビルド依存が増え、ライブラリの依存は増えない | No |
| `embed-model-unidic`、`embed-model-unidic-cwj`、`embed-model-unidic-csj` | [UniDic のモデル](./model_file.md#unidic-のモデル)を埋め込む。`embedded::model("unidic")` と `--model embedded://unidic` で使う。ほかの 2 つの名前でも同じ | No |

このクレートが依存するのは `lindera-segmenter` までで、`lindera` と `lindera-analysis` には依存しません。トークンフィルターのために、`lindera-analysis` が `dependency` feature の裏でこのクレートに依存するためです。

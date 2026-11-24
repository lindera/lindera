# アーキテクチャ

## モジュール構成

```text
lindera-dependency/src/
├── lib.rs                # Public API re-exports
├── morpheme.rs           # Morpheme trait
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
│   ├── model.rs          # ChunkerModel: the table of weights, and its bytes
│   ├── decode.rs         # Viterbi over the two labels
│   └── train.rs          # Training with lindera-crf (train)
├── parser.rs             # Parser
├── parser/
│   ├── features.rs       # Group, FeatureSet: the features of a decision
│   ├── shift_reduce.rs   # The loop over the bunsetsu
│   ├── classifier.rs     # Classifier, LinearClassifier
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
    └── lindera-dependency.rs   # The command (train)
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

`Chunker` は、`ChunkerModel` を使って形態素を文節に区切ります。モデルは、素性の文字列から重みへの表で、学習に使ったスキーマ、列、テンプレートを一緒に持ちます。[文節の区切り](./chunking.md)を参照してください。

### Parser / Classifier

`Parser` は、shift-reduce で各文節の係り先を求めます。1 回の判定の素性は、スキーマを通して作る文字列です。`FeatureSet` がどの群の素性を作るかを決め、`Classifier` が素性に点数を付けます。`LinearClassifier` は、素性の重みとその対の重みを足します。`parser::instances` は、正解のある文から分類器の学習事例を作ります。[係り受けの解析](./parsing.md)を参照してください。

### コマンド

`train` feature を付けると、`lindera-dependency` コマンドがビルドされます。分かち書きには `lindera-segmenter` を使います。

| サブコマンド | 内容 | ページ |
| --- | --- | --- |
| `convert` | 文節のラベルを持つ CoNLL-U のファイルから、学習データを作る | [学習データ](./training_data.md) |
| `evaluate` | 出力を正解と比べる | [評価](./evaluation.md) |
| `split` | 交差検証の分割を書き出す | [評価](./evaluation.md) |
| `train-chunker` | 文節の区切りのモデルを学習する | [文節の区切り](./chunking.md) |
| `chunk` | モデルで、ファイルの形態素を区切り直す | [文節の区切り](./chunking.md) |
| `instances` | 解析器の学習事例を書き出す | [係り受けの解析](./parsing.md) |

コマンドは `lindera-cli` ではなく、このクレートにあります。`lindera-cli` は公開されているので、公開していないクレートに依存できません。

## Feature フラグ

| Feature | 説明 | デフォルト |
| --- | --- | --- |
| （なし） | 型、ラティス形式、スキーマ、文節の区切り、解析器。依存がなく、`wasm32-unknown-unknown` 向けにビルドできる | Yes |
| `train` | `corpus` と `evaluation` のモジュール、`lindera-crf` による区切りの学習、`lindera-dependency` コマンド（分かち書きは `lindera-segmenter`） | No |
| `embed-ipadic` | コマンドに IPADIC を埋め込む。`--dict embedded://ipadic` で使う（`train` を含む） | No |

このクレートが依存するのは `lindera-segmenter` までで、`lindera` と `lindera-analysis` には依存しません。これから作るトークンフィルターで、`lindera-analysis` がこのクレートに依存するためです。

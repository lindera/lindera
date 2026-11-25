# Command Line

The `lindera-dependency` command takes plain text, tokenizes it with a Lindera dictionary, and writes the bunsetsu of each sentence and their dependencies in the [lattice format](./architecture.md#the-lattice-format), which is what `cabocha -f1` writes.

It needs a [model file](./model_file.md) with a chunker and a parser: the [model for IPADIC](./model_file.md#the-model-for-ipadic), which a release of Lindera publishes from 7.1.0 on, or one you [train](./parsing.md#training-a-model).

## Building

The command is behind the `cli` feature, which brings `clap` and `lindera-segmenter` and no training code. `embed-ipadic` embeds IPADIC in it, and `embed-model-ipadic` the [model for IPADIC](./model_file.md#embedding-it), which `--model embedded://ipadic` then names:

```shell
cargo build --release -p lindera-dependency --features cli,embed-ipadic
```

The binary is `target/release/lindera-dependency`. A build with `cli` has two subcommands, `parse` and [`chunk`](./chunking.md); the `train` feature adds the ones that make training data, train and evaluate.

## Parsing text

One line of the input is one sentence:

```shell
echo "彼は赤い花を見た。" | lindera-dependency parse \
    --model ipadic.model --dict embedded://ipadic
```

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

A line that starts with `*` opens a bunsetsu: its number, the number of the bunsetsu it depends on (`-1` for the last one) with the letter `D`, the positions of its [head word and function word](./head_and_function_word.md), and the score of the decision that attached it. The score is 0 where the parser had nothing to decide. The morphemes follow with the features of the dictionary, and `EOS` ends the sentence.

- The text is tokenized with the default settings of Lindera, as the [training data](./training_data.md) is. The tokenizer gives no morpheme for a space.
- A line without a morpheme, such as an empty line, gives `EOS` alone, as in CaboCha.
- The command does not split text into sentences: a line with two sentences is parsed as one.
- A sentence is read, parsed and written before the next one is read, so the command can stand in a pipe. When a sentence fails, what was written before it stays, and the message names the line.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `-m`, `--model` | Required | The model file |
| `-d`, `--dict` | Required for text | The dictionary the text is tokenized with: a directory, or `embedded://ipadic` in a build with `embed-ipadic` |
| `-I`, `--input-layer` | `text` | What the input holds already: `text`, `morphemes` or `bunsetsu` |
| `-i`, `--input` | The standard input | The file to parse |
| `-o`, `--output` | The standard output | The file to write |

## The dictionary and the model

A model is trained on the morphemes of one dictionary, and gives wrong answers on those of another one without failing. The command therefore compares the name of the dictionary it loaded with the name in the model file, and stops when they differ:

```text
Error: embedded://ipadic: the model is for the dictionary "unidic", and the morphemes are of "ipadic"
```

## Input that is tokenized already

`--input-layer` says what the input holds. With `morphemes` and `bunsetsu` the input is in the lattice format, no dictionary is loaded, and `--dict` is refused.

| Layer | The input | What the command does | In CaboCha |
| --- | --- | --- | --- |
| `text` | Plain text, one sentence per line | Tokenizes, segments into bunsetsu, parses | `-I0` |
| `morphemes` | Morphemes in the lattice format; bunsetsu lines are not read | Segments into bunsetsu, parses | `-I1` |
| `bunsetsu` | Morphemes and bunsetsu in the lattice format | Parses the bunsetsu of the input | `-I2` |

```shell
lindera-dependency parse --model ipadic.model --input-layer morphemes \
    --input tokens.lattice --output output.lattice
lindera-dependency parse --model ipadic.model --input-layer bunsetsu \
    --input dev.lattice --output output.lattice
```

The three layers give the same output for a sentence, when the morphemes and the bunsetsu are the ones the command would have made. `bunsetsu` on the answer of a corpus measures the parser alone, and `morphemes` the chunker and the parser together: see [Evaluation](./evaluation.md). With no dictionary to compare, what is checked on a lattice input is that each morpheme has the feature columns the schema of the model reads.

## What is kept of CaboCha's options

| CaboCha | Here |
| --- | --- |
| `-f1`, the lattice format | The only output format |
| `-f0` the tree, `-f2`, `-f3` XML, `-f4` CoNLL | Not kept |
| `-I0`, `-I1`, `-I2`, the input layer | `--input-layer` |
| `-O1`, `-O2`, `-O4`, the output layer | Not kept: `lindera tokenize` writes the morphemes, and `chunk` the bunsetsu |
| `-n`, named entities | Not kept |

Reading the output with CaboCha's own tools has not been tried: they are not built here ([#1433](https://github.com/lindera/lindera/issues/1433)). The output is read back by the reader of this crate, and by [`evaluate`](./evaluation.md).

## Speed

On the text of the test file of UD Japanese-GSD (543 sentences), with the default model and the embedded IPADIC, the command writes about 6,600 sentences per second, from the text to the lattice format. Loading the dictionary and the model takes about 5 ms. The numbers depend on the machine; `scripts/dependency/parse_speed.sh` measures them.

## The `lindera` command

`lindera tokenize` runs the parser as a token filter, in a build with the `dependency` feature: see [Token Filter](./token_filter.md). It writes the result on the tokens of its JSON output, not the lattice format. The feature is in no default build, and works in a checkout only: `lindera-cli` is published on crates.io and cannot depend on a crate that is not, until this crate is published ([#1440](https://github.com/lindera/lindera/issues/1440)).

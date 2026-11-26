# コマンドライン

`lindera-dependency` コマンドは、テキストを受け取り、Lindera の辞書で分かち書きし、各文の文節とその係り受けを[ラティス形式](./architecture.md#ラティス形式)で書き出します。`cabocha -f1` が書くのと同じ形式です。

区切りと解析器の入った[モデルファイル](./model_file.md)が必要です。7.1.0 から Lindera のリリースが公開する [IPADIC のモデル](./model_file.md#ipadic-のモデル)か [UniDic のモデル](./model_file.md#unidic-のモデル)、または自分で[学習した](./parsing.md#モデルを学習する)モデルを使います。

## ビルド

コマンドは `cli` feature の裏にあります。この feature は `clap` と `lindera-segmenter` を入れ、学習のコードは入れません。`embed-ipadic` を付けると、IPADIC を埋め込みます。`embed-model-ipadic` を付けると、[IPADIC のモデル](./model_file.md#埋め込む)を埋め込み、`--model embedded://ipadic` で指せます。UniDic CWJ なら、`embed-model-unidic-cwj` と `--model embedded://unidic-cwj` です。`unidic` と `unidic-csj` も同じで、辞書はディレクトリで渡します。

```shell
cargo build --release -p lindera-dependency --features cli,embed-ipadic
```

バイナリは `target/release/lindera-dependency` です。`cli` のビルドのサブコマンドは、`parse` と [`chunk`](./chunking.md) の 2 つです。`train` feature を付けると、学習データを作る、学習する、評価するサブコマンドが加わります。

## テキストを解析する

入力の 1 行が 1 文です。

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

`*` で始まる行が、文節の始まりです。文節の番号、係り先の文節の番号（最後の文節は `-1`）と文字 `D`、[主辞と機能語](./head_and_function_word.md)の位置、その係りを決めた判定の点数が並びます。解析器が決めることの無かった文節の点数は 0 です。続く行は形態素で、辞書の素性が付きます。`EOS` が文の終わりです。

- テキストは、[学習データ](./training_data.md)と同じく、Lindera の既定の設定で分かち書きします。空白は形態素になりません。
- 空行のように形態素の無い行は、CaboCha と同じく、`EOS` だけになります。
- コマンドは、文章を文に区切りません。2 つの文がある行は、1 つの文として解析します。
- 1 文ずつ読んで、解析して、書きます。そのため、パイプの中で使えます。ある文で失敗すると、そこまでに書いた出力は残り、メッセージがその行を示します。

## オプション

| オプション | 既定 | 意味 |
| --- | --- | --- |
| `-m`、`--model` | 必須 | モデルファイル |
| `-d`、`--dict` | テキストでは必須 | テキストを分かち書きする辞書。ディレクトリか、`embed-ipadic` のビルドでは `embedded://ipadic` |
| `-I`、`--input-layer` | `text` | 入力がすでに持っているもの。`text`、`morphemes`、`bunsetsu` |
| `-i`、`--input` | 標準入力 | 解析するファイル |
| `-o`、`--output` | 標準出力 | 書き出すファイル |

## 辞書とモデル

モデルは 1 つの辞書の形態素で学習したもので、ほかの辞書の形態素を渡しても、失敗せずに誤った答えを返します。そこでコマンドは、読み込んだ辞書の名前をモデルファイルの中の名前と比べ、違えば止まります。

```text
Error: embedded://ipadic: the model is for the dictionary "unidic", and the morphemes are of "ipadic"
```

## 分かち書き済みの入力

`--input-layer` は、入力が何を持っているかを指定します。`morphemes` と `bunsetsu` では、入力はラティス形式で、辞書は読み込みません。`--dict` を付けるとエラーになります。

| 層 | 入力 | コマンドがすること | CaboCha では |
| --- | --- | --- | --- |
| `text` | 1 行 1 文のテキスト | 分かち書きし、文節に区切り、解析する | `-I0` |
| `morphemes` | ラティス形式の形態素。文節の行は読まない | 文節に区切り、解析する | `-I1` |
| `bunsetsu` | ラティス形式の形態素と文節 | 入力の文節を解析する | `-I2` |

```shell
lindera-dependency parse --model ipadic.model --input-layer morphemes \
    --input tokens.lattice --output output.lattice
lindera-dependency parse --model ipadic.model --input-layer bunsetsu \
    --input dev.lattice --output output.lattice
```

形態素と文節がコマンドの作るものと同じであれば、3 つの層は、1 つの文に同じ出力を返します。コーパスの正解に `bunsetsu` を使うと解析器だけを、`morphemes` を使うと区切りと解析器の両方を測れます。[評価](./evaluation.md)を見てください。ラティス形式の入力では、比べる辞書が無いので、各形態素が、モデルのスキーマの読む素性の列を持つことだけを検査します。

## CaboCha のオプションのうち残したもの

| CaboCha | このコマンド |
| --- | --- |
| `-f1`、ラティス形式 | 唯一の出力形式 |
| `-f0` 木、`-f2`、`-f3` XML、`-f4` CoNLL | 無い |
| `-I0`、`-I1`、`-I2`、入力の層 | `--input-layer` |
| `-O1`、`-O2`、`-O4`、出力の層 | 無い。形態素は `lindera tokenize` が、文節は `chunk` が書く |
| `-n`、固有表現 | 無い |

出力は、CaboCha の道具で読めます（[#1433](https://github.com/lindera/lindera/issues/1433)）。`cabocha-system-eval` は、この出力で [`evaluate`](./evaluation.md) と同じ数を数えます。`cabocha -I 2` は、文節つきの入力として受け取ります。

## 速度

UD Japanese-GSD の test のテキスト（543 文）で、既定のモデルと埋め込んだ IPADIC を使うと、コマンドは、テキストからラティス形式まで、毎秒約 6,600 文を書き出します。辞書とモデルの読み込みは約 5 ミリ秒です。数字はマシンによって変わります。`scripts/dependency/parse_speed.sh` で測れます。

## `lindera` コマンド

`lindera tokenize` は、解析器をトークンフィルターとして動かします。[トークンフィルター](./token_filter.md)を見てください。結果は、ラティス形式ではなく、JSON の出力のトークンに書かれます。`lindera` コマンドは、7.1.0 から、既定のビルドにフィルターを持ちます。

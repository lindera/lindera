# モデルファイル

モデルファイルは、1 つの辞書を解析するのに要るものを 1 つにまとめたファイルです。辞書の名前、その[スキーマ](./head_and_function_word.md)、学習した部分が入ります。部分は、[文節の区切り](./chunking.md)と[解析器](./parsing.md)です。

モデルは 1 つの辞書に属します。辞書の素性の列と、辞書による分かち書きに依存するので、ファイルは、どの辞書のためのものかを自分で持ち、1 つのファイルが 1 つの辞書に対応します。

[IPADIC のモデル](#ipadic-のモデル)は、7.1.0 から、Lindera のリリースが公開します。[`train-chunker`](./chunking.md#モデルを学習する) は区切りを入れたモデルファイルを、[`train-parser`](./parsing.md#モデルを学習する) は解析器だけか、両方を入れたモデルファイルを書き出します。

## モデルを読む

`Model` は、ファイルをその場で読みます。バイト列を持つ型について総称で、表をどこにも写しません。

```rust
use lindera_dependency::model::Model;

// Bytes read from a file: the model owns them.
let model = Model::new(std::fs::read("ipadic.model")?)?;

assert_eq!(model.dictionary(), "ipadic");
let chunker = model.chunker().expect("the file has a chunker");
let parser = model.parser().expect("the file has a parser");
```

| バイト列を持つもの | 書き方 |
| --- | --- |
| メモリに読み込んだファイル | `Model::new(std::fs::read(path)?)` で `Model<Vec<u8>>` になる |
| プログラムに埋め込んだファイル | `Model::new(include_bytes!("ipadic.model").as_slice())` で `Model<&'static [u8]>` になる |
| メモリマップ | `AsRef<[u8]>` を実装する型なら何でも `Model::new(map)` に渡せる。`memmap2` クレートのマップなど。このクレートは依存を持たず、自分ではマップしない |

ファイルの全体は、`Model::new` の中で 1 回だけ検査します。この版のモデルファイルでないバイト列は `Error::Model` になり、その後は何も失敗せず、panic もしません。数をポインター越しに読まないので、バイト列はどのアドレスにあってもよく、境界に揃っている必要もありません。同じコードが `wasm32-unknown-unknown` で動きます。

`Model::chunker` は、モデルのバイト列の上で重みを読む `Chunker` を返します。ファイルに区切りの部分が無ければ `None` です。`Model::parser` は、同じように `Parser` を返します。`Model::analyze` は、1 回の呼び出しで文を区切って解析します。`Model` は、バイト列がそうであれば `Send + Sync` で、1 つのモデルを複数のスレッドで使えます。

## 辞書の照合

モデルを別の辞書の形態素に使うと、検査しない限り、エラーにならずに誤った答えが出ます。検査は 2 つあります。

**名前。** 形態素の辞書を知っている呼び出し側が、その名前をモデルの名前と比べます。

```rust
model.check_dictionary("ipadic")?;
assert!(model.check_dictionary("unidic").is_err());
```

```text
the model is for the dictionary "ipadic", and the morphemes are of "unidic"
```

**素性の列の数。** モデルのスキーマが読む列より少ない列しか持たない形態素は、区切りや解析のときに `Error::Dictionary` で断ります。IPADIC のスキーマは 7 列を読みます。品詞の 4 階層、活用型、活用形、原形です。

```rust
use lindera_dependency::lattice::LatticeMorpheme;

// Three feature columns: not a morpheme of IPADIC.
let morphemes = [LatticeMorpheme::new("彼", "名詞,代名詞,一般")];
let error = chunker.chunk(&morphemes).unwrap_err();
println!("{error}");
```

```text
the morpheme 1 ("彼") has fewer than the 7 feature columns the schema reads: it is not of the dictionary of the model
```

2 つ目の検査は、列の数が足りている別の辞書の形態素を見つけられません。UniDic の形態素は IPADIC より列が多く、列の意味も違います。これを見分けられるのは名前だけなので、辞書を知っている呼び出し側は名前を照合してください。

## モデルを書く

`ModelBuilder` は、辞書の名前、スキーマ、部分からファイルを書きます。区切りの部分は `ChunkerModel` から、解析器の部分は `ParserModel` から作ります。どちらも学習の結果です（[区切りの学習](./chunking.md#モデルを学習する)、[解析器の学習](./parsing.md#モデルを学習する)）。次の区切りは、手で作ったものです。

```rust
use std::collections::HashMap;

use lindera_dependency::chunker::{ChunkerModel, Column, Template};
use lindera_dependency::model::{Model, ModelBuilder};
use lindera_dependency::Schema;

// A chunker made by hand, with one weight: a morpheme after a particle
// starts a bunsetsu.
let mut features = HashMap::new();
features.insert("U11:助詞-係助詞".to_string(), [1.0, 0.0]);
let transitions = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.5], [0.0, 0.0, 0.5]];
let chunker = ChunkerModel::new(
    Schema::ipadic(),
    vec![Column::Surface, Column::PartOfSpeech],
    Template::cabocha(),
    transitions,
    features,
)?;

let bytes = ModelBuilder::new("ipadic", Schema::ipadic())?
    .chunker(&chunker)?
    .to_bytes();
std::fs::write("chunker.model", &bytes)?;

let model = Model::new(bytes)?;
assert_eq!(model.dictionary(), "ipadic");
```

同じ部分からは、同じバイト列ができます。ファイルのスキーマと違うスキーマで学習した部分は断ります。

`ModelBuilder::parser` は、`ParserModel` と、重みの書き方（`Quantization::F32`、`I16`、`I8`）を受け取ります。`ModelBuilder::from_model` は、すでにあるファイルの名前、スキーマ、部分から始めて、部分を足せるようにします。`train-parser --chunker` は、これで区切りを写します。

## IPADIC のモデル

このプロジェクトは、IPADIC のモデルを 1 つ学習します。区切りと解析器の両方が入ります。7.1.0 から、Lindera のリリースが [`build_model.sh`](#作り直す) でモデルを作り、その版のページに、辞書と並べて置きます。`v<version>` の `lindera-dependency-ipadic-<version>.zip` です。モデルは、辞書と同じく、リリースの版を持ち、そのリリースに属します。Lindera 7.1.0 のプログラムは、7.1.0 のモデルを使います。

[`lindera download dependency-ipadic`](../lindera-cli/commands.md#係り受け解析のモデルのダウンロード) が、コマンドのバージョンのアーカイブをインストールし、モデルファイルのパスを表示します。

アーカイブには、ディレクトリ `lindera-dependency-ipadic` が 1 つ入っていて、中にファイルが 3 つあります。

| ファイル | 中身 |
| --- | --- |
| `dependency.model` | モデルファイル |
| `model.json` | モデルの名前（`dependency-ipadic`）、辞書（`ipadic`）、Lindera の版（`lindera_version`） |
| `NOTICE.txt` | モデルのライセンス、何から作ったか、IPADIC の表示 |

| | |
| --- | --- |
| 辞書 | IPADIC（`ipadic`）。`lindera-ipadic` クレートが mecab-ipadic 2.7.0 から作るもの |
| 学習データ | [UD Japanese-GSD](https://github.com/UniversalDependencies/UD_Japanese-GSD)（c662dc9）の train のファイルを、IPADIC で[変換](./training_data.md)したもの。7,024 文 |
| 設定 | [`train-chunker`](./chunking.md#モデルを学習する) と [`train-parser`](./parsing.md#モデルを学習する) の既定 |
| 大きさ | 6,258,548 バイト |
| ライセンス | MIT。表示を添えます。[モデルのライセンス](#モデルのライセンス)を見てください |

使うのは train のファイルだけです。UD Japanese-GSD の dev と test、UD Japanese-PUD は学習で見ておらず、設定の選択にも使っていません。そのため、これらで測った[精度](./parsing.md#精度)は、このファイルそのものの精度です。

### 埋め込む

`lindera-dependency` の feature `embed-model-ipadic` が、モデルをプログラムに埋め込みます。`lindera`、`lindera-cli`、各バインディングは、これを `embed-dependency-ipadic` として転送します。埋め込んだモデルは、埋め込んだ辞書と同じ書き方で指します。

```rust
use lindera_dependency::embedded;

// The bytes of the program, read in place.
let model = embedded::model("ipadic")?;
assert_eq!(model.dictionary(), "ipadic");
```

[コマンド](./command_line.md)には `--model embedded://ipadic` を、[トークンフィルター](./token_filter.md)には `model: "embedded://ipadic"` を渡します。feature の無いビルドは、その feature の名前を挙げたエラーを返します。

ビルドは、モデルを学習しません。学習にはコーパスが要るためです。ビルドがモデルをどこから取るかは、クレートをどこから取ったかで決まります。

| ビルド | 埋め込むモデル |
| --- | --- |
| crates.io のクレート | そのリリースのアーカイブ。クレートが `prebuilt.json` に SHA-256 で固定している。ビルドは、ダウンロードして照合する。辞書のクレートが[辞書](../development/dictionary_release.md)にするのと同じ |
| リポジトリ（`main`、pull request、git の依存） | [`LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE`](../getting_started/environment_variables.md#lindera_build_dependency_model_ipadic_file) が指すファイル。リポジトリには `prebuilt.json` が無く、この変数の無いビルドは、その旨のエラーで止まる |
| docs.rs | 空のファイル。`embedded::model` はエラーを返す |

この変数を設定すると、公開済みのクレートも、固定しているアーカイブの代わりに、そのファイルを埋め込みます。

```shell
scripts/dependency/build_model.sh
export LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE=$PWD/scripts/dependency/data/model/lindera-dependency-ipadic/dependency.model
cargo build --release -p lindera-cli --features embed-ipadic,embed-dependency-ipadic
```

### 作り直す

1 つのスクリプトが、コーパスとリポジトリのソースからファイルを作ります。

```shell
scripts/dependency/build_model.sh
```

スクリプトは、UD Japanese-GSD を固定の commit で取り、`lindera-dependency` コマンドを、名指しした toolchain と `Cargo.lock` の依存でビルドします。続けて、train のファイルを変換し、区切りと解析器を学習して、`scripts/dependency/data/model` に、アーカイブのディレクトリとアーカイブを書きます。ビルドの後は 15 秒ほどで終わります。最後に、Lindera の commit を表示し、続けて、ほかに使ったものと作ったものを表示します。

```text
corpus: UD Japanese-GSD c662dc96ed5ee5ac0fc8b5e44261557e431f50e6
toolchain: rustc 1.100.0 (0ef1bc386 2026-11-09)
platform: x86_64-unknown-linux-gnu, glibc 2.43
training data: 5075c95f2518b6af791e6a9ea2bd5f9f63eeaa5b316f3801d8068d71436e5064  train.lattice
chunker: 74762e9b4f23d1dd06a304238006480264e0fea70b7a3a5c9e647c176dad59c3  chunker.model
model: 793b22e6ef2d92abf0fd91f282942acc6ebd568c22b930770ff2ff57573fbe46  lindera-dependency-ipadic/dependency.model (6258548 bytes)
archive: f688c9077d58c3ecd9ad6d770d6074f326ea8ebaccc424319d8dd9886873e715  lindera-dependency-ipadic-7.0.0.zip
```

`--check <sha256>` を付けると、作ったファイルの SHA-256 が違うときに、スクリプトが失敗します。未コミットの変更がある作業ツリーでは、スクリプトは動きません。表示する commit が、ファイルの元になったソースと一致するようにするためです。

ソース、コーパス、toolchain、プラットフォームが同じなら、同じバイト列になります。学習は、時刻にも、スレッドの数にも、ハッシュ表の順序にも依存しません。別のプラットフォームで同じバイト列になるかは、分かっていません。区切りの学習は、プラットフォームの数学ライブラリの `exp` と `ln_1p` を呼び、その最後の 1 ビットはライブラリによって違うことがあります。ファイルの表示（`NOTICE.txt`）に、作ったプラットフォームを書いてあります。リリースのモデルは、リリースのランナーが作るので、ほかの端末で作ったものとバイト列が違うことがあります。そのアーカイブの SHA-256 は、公開したクレートの `prebuilt.json` にあります。リリースは、作ったモデルを dev・test・PUD で測り、数がこのドキュメントの値と違えば止まります（[辞書のリリース](../development/dictionary_release.md#lindera-dependency-のモデル)）。

### モデルのライセンス

モデルファイルは、Lindera のコードと同じ MIT ライセンスで配布します。ファイルに添える `NOTICE.txt` に、ライセンスの本文、UD Japanese-GSD への謝辞、IPADIC の表示が入っています。以下は、このプロジェクトでの扱い方であり、法的な助言ではありません。

**問い。** UD Japanese-GSD のライセンスは CC BY-SA 4.0 です。このライセンスは、元の素材に「由来する、または基づく」素材（Adapted Material）を、同じライセンスで共有することを求めます。ある著作物で学習したモデルがそれに当たるかは、ライセンスにも、Creative Commons の FAQ にも書かれていません。

**モデルが持つもの。** 素性の名前と、その重みです。名前は、品詞、活用形、1 つの形態素の表層のどれかから作るので、学習した文に出てくる単語は、1 語ずつファイルに入っています。文は入っていません。注釈も入っていません。文節も、係り先もありません。モデルは、テキストを生成しません。

**立場。** このプロジェクトは、こうしたモデルはコーパスの改変物ではなく、継承（share-alike）の条件は及ばない、という立場を取ります。根拠は次のとおりです。

- 文化庁の[「AI と著作権に関する考え方について」](https://www.bunka.go.jp/seisaku/bunkashingikai/chosakuken/pdf/94037901_01.pdf)（2024 年 3 月）: 「学習済モデルは、学習に用いられた著作物の複製物とはいえない場合が多い」。学習データと類似性のあるものを高確率で生成する状態にある場合などは、複製物と評価されることがある。この文書は、自身に法的な拘束力はないとしている。
- Szkalej・Senftleben の [Mapping the Impact of Share Alike/Copyleft Licensing on Machine Learning and Generative AI](https://www.ivir.nl/publicaties/download/Share-Alike-and-ML.pdf)（2024 年）: 学習済みのモデルは、学習に使った著作物の保護される痕跡を含まないかぎり、Adapted Material に当たる見込みは低い。
- Creative Commons の [Using CC-licensed Works for AI Training](https://creativecommons.org/wp-content/uploads/2025/05/Using-CC-licensed-Works-for-AI-Training.pdf)（2025 年）: モデルを学習データと同じライセンスで公開するのは「最も保守的な方法」で、従えば「ほぼ確実に、過剰な遵守になる」。
- CC BY-SA のテキストで学習したモデルが、制約の緩いライセンスで公開されている。Wikipedia で学習した [BERT](https://github.com/google-research/bert) のモデルは Apache 2.0。

**不確かな点。** 学習済みのモデルが学習データの改変物かどうかを判断した裁判例は、ありません。逆を選んだ例もあります。同じコーパスで学習した [spaCy の日本語のモデル](https://github.com/explosion/spacy-models/releases/tag/ja_core_news_sm-3.8.0)は、CC BY-SA 4.0 です。この立場が誤りだと分かったら、それ以降は CC BY-SA 4.0 で配布します。ファイルそのものは変わりません。

**利用者がすること。**

| 使い方 | 必要なこと |
| --- | --- |
| モデルでテキストを解析する | なし。解析の結果に、このプロジェクトは条件を付けません |
| ファイルを配布する、ファイルを埋め込んだプログラムを配布する | `NOTICE.txt` を添える。MIT ライセンスの本文と、IPADIC の表示が入っています |
| モデルを変更して配布する | 同じ |

## 形式

版は 1 です。数はすべてリトルエンディアンです。文字列は、バイト数を `u32` で書き、その後に UTF-8 を続けます。

| 欄 | 型 | 中身 |
| --- | --- | --- |
| マジックナンバー | 8 バイト | `LDMODEL` と 0 のバイト |
| 版 | `u32` | 1 |
| 辞書 | 文字列 | 辞書の名前。空ではない |
| スキーマ | | 品詞・活用・原形の列。機能語、主辞にならないもの、助詞、表層を使う機能語のパターン。記号 |
| 部分の数 | `u32` | |
| 部分ごとに | `u32`、`u64`、`u64` | 種類、ファイルの先頭からの位置、長さ |
| 部分 | | 一覧の順に、すき間なく、ファイルの終わりまで |

区切りの部分の種類は 1、解析器の部分の種類は 2 で、この順に並びます。ほかの種類の部分があるファイル、同じ種類が 2 つあるファイル、部分がすき間なく並んでいないファイル、最後の部分の後にバイトが残るファイルは断ります。ほかの版のファイルも断ります。

### 区切りの部分

| 欄 | 型 | 中身 |
| --- | --- | --- |
| 列の数 | `u32` | |
| 列 | 各 1 バイト | テンプレートが読むもの。0 は表層、1 は品詞、2 は活用型、3 は活用形、4 は原形 |
| テンプレート | 文字列 | CRF++ の形式 |
| 遷移 | 9 × `f64` | `[from][to]` の順。0 は `from` では文頭、`to` では文末。1 は `B`、2 は `I` |
| 素性の文字列 | 表 | それぞれに `f64` が 2 つ。`B` の重み、`I` の重みの順 |

### 解析器の部分

| 欄 | 型 | 中身 |
| --- | --- | --- |
| 素性の集合 | 文字列 | 素性の群。`distance,source,target,left,right,children,gap,after` の形 |
| 量子化 | 1 バイト | 重みの書き方。0 は `f32`、1 は `i16`、2 は `i8` |
| 対 | 1 バイト | 0 は対ごとの重み、1 は対をハッシュで割り当てる表 |
| バイアス | `f64` | |
| 尺度 | `f64` | 書いてある重みに掛ける数。0 より大きく、`f32` では 1 |
| 素性の文字列 | 表 | それぞれに重み。対をハッシュで割り当てる場合は、その後に `u32` のキー |
| 対の重み | | 下を参照 |

1 回の判定の点数は、バイアスに、その素性と素性の対の重み（書いてある値）の合計に尺度を掛けたものを足した値です。素性の番号は、表の中の順番です。

**対をハッシュで割り当てる場合**: ビット数を `u32` で書き（1 から 30）、その後に 2^bits 個の重みが続きます。素性のキーは、その文字列のハッシュで、表のハッシュと同じものです。対の場所は 2 つのキーから決めます。小さいほうを 64 ビットの数の上位の半分に置き、その数を混ぜて（30 ビット右にずらした自分との XOR、`0xbf58476d1ce4e5b9` との積、27 ビットずらした XOR、`0x94d049bb133111eb` との積）、上位のビットを場所にします。同じ場所に当たった対は、その重みを共有します。

**対ごとに重みを持つ場合**: 対の数を `u64` で書きます。次に、素性ごとと最後にもう 1 つ、その素性の対（その素性が小さいほうの番号になる対）がどこから始まるかを `u32` で書きます。続いて、各対の大きいほうの番号を `u32` で、素性ごとに並べて書き、最後に各対の重みを書きます。

### 表

表は、文字列から同じ幅の値への対応で、ファイルの中の位置のまま引きます。

| 欄 | 型 | 中身 |
| --- | --- | --- |
| 文字列の数 | `u32` | |
| 値の幅 | `u32` | バイト数 |
| 枠の数 | `u32` | 文字列の数より大きい 2 の冪 |
| 位置 | （数 + 1）× `u32` | 各文字列が、下の文字列の並びのどこから始まるか。最後は全体の長さ |
| 文字列 | | UTF-8 を続けて並べる。バイト列の順に並べ、同じものは 1 つだけ |
| 値 | 数 × 幅 | 文字列の順 |
| 枠 | 枠の数 × `u32` | 文字列の上のハッシュ表 |

枠は、空なら 0、そうでなければ文字列の番号に 1 を足した値です。文字列は、ハッシュが指す枠から始めて、その後の最初の空いた枠に入ります。最後の枠の次は最初の枠です。ハッシュは、文字列のバイト列の 64 ビットの FNV-1a を 32 ビットに畳み（上位の半分と下位の半分の XOR）、枠の数で割った余りです。文字列は並びの順に入れ、表は 1 つの文字列に 2 つ以上の枠を持つように書きます。

文字列を並べ替えてあるのは、同じ文字列から必ず同じバイト列ができるようにするためです。引くときは枠を使います。

## 大きさと速さ

UD Japanese-GSD の train と IPADIC で学習した、既定のモデル（区切りと解析器）の値です。test の 542 文で測りました。

| | |
| --- | --- |
| 両方の部分を入れたファイルの大きさ | 6,258,548 バイト |
| そのうち区切りを、単独のファイルにした大きさ | 261,851 バイト |
| 読み込みの時間（`Model::new`。すべての検査を含む） | 約 2.7 ms |
| 区切り | 毎秒 約 20,400 文 |
| 正解の文節の解析 | 毎秒 約 16,700 文 |
| 区切りと解析 | 毎秒 約 8,700 文 |

対の表は 1 バイトの重みが 2^22 個で、ファイルの 4.2 MB を占めます。残りのほとんどは、解析器の素性の文字列 62,380 個です。

対の重みの持ち方で変わるのは、大きさと速さで、精度ではありません（[既定の選び方](./parsing.md#既定の選び方)を参照）。

| 対 | 重み | ファイルの大きさ | 解析、毎秒の文の数 |
| --- | --- | --- | --- |
| 2^22 個の重みにハッシュで割り当てる | 1 バイト | 6,258,548 バイト | 約 16,700 |
| 2^22 個の重みにハッシュで割り当てる | 4 バイト | 19,028,600 バイト | 約 13,400 |
| 対ごとの重み | 1 バイト | 24,514,588 バイト | 約 2,700 |
| 対ごとの重み | 4 バイト | 38,162,161 バイト | 約 2,700 |

同じデータで学習した区切りだけ（素性の文字列は 4,325 個）を、train の 7,024 文で測った値です。

| | |
| --- | --- |
| ファイルの大きさ | 261,851 バイト |
| 読み込みの時間（`Model::new`。すべての検査を含む） | 約 0.2 ms |
| ファイルの区切りでの区切り | 毎秒 約 16,896 文 |
| 同じ区切りをメモリに持った場合（標準ライブラリのハッシュ表） | 毎秒 約 18,223 文 |

AMD Ryzen AI Max+ 395 の 1 コアで、Rust 1.100.0 を使って測りました。[`scripts/dependency/model_speed`](https://github.com/lindera/lindera/tree/main/scripts/dependency/model_speed) と `scripts/dependency/parser_experiments.sh speed` で測り直せます。

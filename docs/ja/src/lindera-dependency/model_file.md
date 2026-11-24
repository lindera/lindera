# モデルファイル

モデルファイルは、1 つの辞書を解析するのに要るものを 1 つにまとめたファイルです。辞書の名前、その[スキーマ](./head_and_function_word.md)、学習した部分が入ります。今ある部分は[文節の区切り](./chunking.md)だけで、解析器は後から入ります。

モデルは 1 つの辞書に属します。辞書の素性の列と、辞書による分かち書きに依存するので、ファイルは、どの辞書のためのものかを自分で持ち、1 つのファイルが 1 つの辞書に対応します。

配布しているモデルは、まだありません。モデルファイルは [`train-chunker`](./chunking.md#モデルを学習する) が書き出します。

## モデルを読む

`Model` は、ファイルをその場で読みます。バイト列を持つ型について総称で、表をどこにも写しません。

```rust
use lindera_dependency::model::Model;

// Bytes read from a file: the model owns them.
let model = Model::new(std::fs::read("chunker.model")?)?;

assert_eq!(model.dictionary(), "ipadic");
let chunker = model.chunker().expect("the file has a chunker");
```

| バイト列を持つもの | 書き方 |
| --- | --- |
| メモリに読み込んだファイル | `Model::new(std::fs::read(path)?)` で `Model<Vec<u8>>` になる |
| プログラムに埋め込んだファイル | `Model::new(include_bytes!("chunker.model").as_slice())` で `Model<&'static [u8]>` になる |
| メモリマップ | `AsRef<[u8]>` を実装する型なら何でも `Model::new(map)` に渡せる。`memmap2` クレートのマップなど。このクレートは依存を持たず、自分ではマップしない |

ファイルの全体は、`Model::new` の中で 1 回だけ検査します。この版のモデルファイルでないバイト列は `Error::Model` になり、その後は何も失敗せず、panic もしません。数をポインター越しに読まないので、バイト列はどのアドレスにあってもよく、境界に揃っている必要もありません。同じコードが `wasm32-unknown-unknown` で動きます。

`Model::chunker` は、モデルのバイト列の上で重みを読む `Chunker` を返します。ファイルに区切りの部分が無ければ `None` です。`Model` は、バイト列がそうであれば `Send + Sync` で、1 つのモデルを複数のスレッドで使えます。

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

`ModelBuilder` は、辞書の名前、スキーマ、部分からファイルを書きます。区切りの部分は、[学習](./chunking.md#モデルを学習する)の結果である `ChunkerModel` から作ります。

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

同じ部分からは、同じバイト列ができます。ファイルのスキーマと違うスキーマで学習した区切りは断ります。

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

区切りの部分の種類は 1 です。ほかの種類の部分があるファイル、同じ種類が 2 つあるファイル、部分がすき間なく並んでいないファイル、最後の部分の後にバイトが残るファイルは断ります。ほかの版のファイルも断ります。

### 区切りの部分

| 欄 | 型 | 中身 |
| --- | --- | --- |
| 列の数 | `u32` | |
| 列 | 各 1 バイト | テンプレートが読むもの。0 は表層、1 は品詞、2 は活用型、3 は活用形、4 は原形 |
| テンプレート | 文字列 | CRF++ の形式 |
| 遷移 | 9 × `f64` | `[from][to]` の順。0 は `from` では文頭、`to` では文末。1 は `B`、2 は `I` |
| 素性の文字列 | 表 | それぞれに `f64` が 2 つ。`B` の重み、`I` の重みの順 |

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

UD Japanese-GSD の train と IPADIC で学習した、既定の区切りのモデル（素性の文字列は 4,325 個）の値です。

| | |
| --- | --- |
| ファイルの大きさ | 261,851 バイト |
| 読み込みの時間（`Model::new`。すべての検査を含む） | 約 0.2 ms |
| ファイルの区切りでの区切り | 毎秒 約 16,896 文 |
| 同じ区切りをメモリに持った場合（標準ライブラリのハッシュ表） | 毎秒 約 18,223 文 |

AMD Ryzen AI Max+ 395 の 1 コアで、Rust 1.100.0 を使い、train の 7,024 文で測りました。[`scripts/dependency/model_speed`](https://github.com/lindera/lindera/tree/main/scripts/dependency/model_speed) で測り直せます。ファイルの大部分を占めることになる解析器の部分は、まだありません。モデル全体の大きさと速さは、それと一緒に測ります。

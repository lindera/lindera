# linderaが保持しているバイナリファイルについて

LinderaがTokenizerの内部処理で利用しているバイナリファイルがいくつかあります。
これらのファイルがそれぞれどんなデータが入っていて、どんな場面で利用されているのかについて、まとめておきます。
また、これらのファイルは各辞書のビルダー(例 : lindera-ipadic-builder)によって生成されるファイルでもあります。

## char_def.bin

文字種のカテゴリーマッピングファイル。

### データ構造

`CharactereDefinitions`構造体を`bincode`でシリアライズしたもの

### データの内容

`CharacterDefinitions`は3種類のデータを保持。

* category_names : カテゴリ名のベクタ
* category_definitions : カテゴリデータのベクタ
    * カテゴリデータは以下の通り
        * invoke
        * group
        * length
* mapping : UCS2のコードポイントをカテゴリにマッピングするためのテーブル 
    
## matrix.mtx

Tokenizer内部でビタビアルゴリズムで利用するコストの連接表。

### データ構造

[`matrix.def`](https://taku910.github.io/mecab/dic-detail.html) (2次元配列)を`[i16]`のベクタ(1次元配列)に詰め込み、LittleEndianでシリアライズしたもの

### データの内容

* 0番目：前件サイズ(forward_size)
* 1番目：後件サイズ(backward_size)
* 2番目以降：[左文脈ID][右文脈ID]=コストという2次元配列を1次元配列化したもの。値としてはコストが入る。

> 内部実装では、backward_sizeを利用して1次元配列化している

## unk.bin

未知語の定義ファイル。
`char.def`で定義された文字種のカテゴリを見出し語として、定義した辞書ファイル。

### データ構造

`UnknownDictionary`構造体を`bincode`でシリアライズしたもの

### データの内容

* category_references : `char_def.bin`のカテゴリ名に対する、`word_id`(=`unk.def`の行番号)のベクタ(2次元ベクタ)。
* costs : `WordEntry`構造体のベクタ。`unk.def`の行数がベクタの長さ。
    * word_id(`u32`) : 使用しないので、`u32::MAX`
    * cost_id(`u16`)  : 未知語の左文脈ID(左右の文脈IDが同一のものだけを利用)
    * word_cost(`i16`) : 未知語のコスト 

## dict.da

辞書の単語一覧をルックアップするためのDoubleArrayTrieデータ構造。
キー(`str`)を元に、値(`u32`)を取り出す。
後述する`dict.vals`と合わせて利用する。

### データ構造

[`yada`](https://github.com/takuyaa/yada/)により生成されるDoubleArrayTieをシリアライズしたバイナリ。

### データの内容

* キー(str) : 辞書の見出し語 
* 値(u32) : `dict.vals`にある値を取り出すための`offset_len`をエンコードしたもの。
    * len : 下位5ビット
    * offset : 上位27ビット

## dict.vals

辞書のデータのうち、ビタビアルゴリズムで使用する必要最低限のデータ。
見出し語に対応するデータの取り出しには、`dict.da`が必要。

### データ構造

辞書のデータのうち、`WordEntry`構造体のデータをバイト配列にシリアライズしたデータ。
バイト配列には、見出し語に対応する`Vec<WordEntry>`が、見出し語の辞書順(RustのBTreeMap)で保存されている。

### データの内容

`WordEntry`構造体(`WordId`に関しては、フラグは辞書の再読み込み時に付与するため、バイナリデータには保存しない)
* word_id(`u32`) :  見出し語ID(Rustで0始まりで付番したもの。辞書ファイルのIDとは異なる)
* word_cost(`i16`) : 見出し語のコスト 
* cost_id(`u16`)  : 見出し語の左文脈ID(左右の文脈IDが同一のものだけを利用)

## dict.wordsidx

`dict.words`をルックアップするためのオフセットを格納。

### データ構造

見出し語IDの昇順に`dict.words`のオフセット(`u32`)をバイト配列にしてシリアライズしたバイナリ。

### データの内容

* `dict.words`のオフセット(`u32`)

## dict.words

辞書の見出し語、コスト以外の情報(品詞、読みなどの付加情報)を保持するデータ構造。
`Vec<String`が見出し語IDの昇順に保存されている。

### データ構造

`bincode`でシリアライズした`Vec<String>`のバイト配列。

### データの内容

* `Vec<String>` : 辞書ごとに保持している項目が異なるので、辞書ごとに何が入っているか確認が必要。


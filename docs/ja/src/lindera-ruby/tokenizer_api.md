# Tokenizer API

## TokenizerBuilder

`Lindera::TokenizerBuilder` はビルダーパターンを使用して `Tokenizer` インスタンスを設定・構築します。

### コンストラクタ

#### `Lindera::TokenizerBuilder.new`

デフォルト設定で新しいビルダーを作成します。

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
```

#### `Lindera::TokenizerBuilder.from_file(file_path)`

JSON ファイルから設定を読み込み、新しいビルダーを返します。これはクラスメソッドであり、既存のインスタンスに対してチェーンするものではありません。

```ruby
builder = Lindera::TokenizerBuilder.from_file('config.json')
```

### 設定メソッド

#### `set_mode(mode)`

トークナイズモードを設定します。`mode` には [`Lindera::Mode`](#mode) を渡します：

- `Lindera::Mode.normal` -- 標準的なトークナイズ（デフォルト）
- `Lindera::Mode.decompose(penalty = nil)` -- [`Lindera::Penalty`](#penalty) の長さのペナルティに従って、複合語をより小さな単位に分解（`nil` では既定のペナルティ）

`Lindera::Mode` でない値（モード文字列 `'normal'`、Symbol、`nil` など）を渡すと `TypeError` が発生します。実行時に読み込んだモード名を `Mode` に変換するには [`Lindera::Mode.from_str`](#linderamodefrom_strname) を使います。

```ruby
builder.set_mode(Lindera::Mode.normal)
builder.set_mode(Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, nil)))
```

#### `set_dictionary(path)`

システム辞書のパスまたは URI を設定します。

```ruby
# 埋め込み辞書を使用
builder.set_dictionary('embedded://ipadic')

# 外部辞書を使用
builder.set_dictionary('/path/to/dictionary')
```

#### `set_user_dictionary(uri)`

ユーザー辞書の URI を設定します。

```ruby
builder.set_user_dictionary('/path/to/user_dictionary.csv')
```

#### `set_keep_whitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。残した空白はラティスにも残るため、他のトークンも、MeCab と同様に空白をラティス上で読み飛ばすデフォルトの出力と変わることがあります。空白を出力せずにラティスにだけ残すには、代わりに [`set_skip_whitespace(false)`](#set_skip_whitespaceskip) を使います。

```ruby
builder.set_keep_whitespace(true)
```

#### `set_skip_whitespace(skip)`

空白をラティス上で読み飛ばすかどうかを設定します。`skip` の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.skip_whitespace` と同じです：

- `true` -- 空白を読み飛ばし、MeCab と同様に空白の前後の語を直接接続します。
- `false` -- 空白を独立したノード（空白の見出し語か `SPACE` の未知語）としてラティスに残します（v7 より前の動作）。空白は引き続き出力から除外されます。
- `nil` -- 辞書の `metadata.json` のデフォルトです。SudachiDict 以外では読み飛ばします。以前の呼び出しを取り消すときに使います。

ここで設定した値は、辞書のデフォルトと、`TokenizerBuilder.from_file` で読み込んだ `skip_whitespace` より優先されます。空白トークンを残すとき（`set_keep_whitespace(true)`）は効果がありません。残した空白は常にラティスに残るためです。それ以外の値は、`set_keep_whitespace` と同じく Ruby の真偽値の規則で扱われ、`false` と `nil` 以外は読み飛ばしを有効にします。

たとえば IPADIC では、`Google が 新しい` の `が` は、デフォルトでは助詞（`助詞`）、`set_skip_whitespace(false)` では接続詞（`接続詞`）と解析されます。詳しくは [Segmenter](../lindera/segmenter.md#空白文字の扱い) を参照してください。

```ruby
# v7 より前と同じく、空白をラティスに残す
builder.set_skip_whitespace(false)
```

#### `set_space_penalty(value)`

韓国語向けの左側空白ペナルティ（left-space penalty）を設定します。直前に空白がある候補のうち、本来は前の語に空白なしで付く品詞（助詞、語尾など）の候補にコストを加算します。`value` の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.space_penalty` と同じです：

- `nil` -- デフォルト。辞書が `metadata.json` に同梱するルールがあればそれを使います（ko-dic は mecab-ko-dic のルールを同梱し、他の同梱辞書はルールを持ちません）。以前の呼び出しを取り消すときにも使います。
- `false` -- ペナルティをオフにします。
- `true` -- 辞書のルールを要求します。ルールを同梱しない辞書では `build` が `RuntimeError` を発生させます。
- 文字列キーのハッシュ `{ 'rules' => [{ 'pos' => [...], 'cost' => n }, ...] }` -- 辞書のルールの代わりにこのルールを適用します。先頭品詞タグが `pos` に含まれる候補に `cost` を加算し、最初に一致したルールが使われます。

これらの形式に当てはまらない値（`1`、`'false'`、`{ 'rules' => 'JKS' }` など）では `RuntimeError` が発生します。変換できない値では `TypeError` が発生します。該当するのは Symbol（ハッシュのキーや要素に含まれる場合も含むため、`%i[JKS]` ではなく `%w[JKS]` と書きます）、有限でない `Float`（NaN や Infinity）、64 ビットの範囲を超える `Integer`、および `nil`・`true`・`false`・`Integer`・`Float`・`String`・`Array`・`Hash` 以外のオブジェクトです。

```ruby
# ペナルティをオフにする
builder.set_space_penalty(false)

# 明示的なルール
builder.set_space_penalty({
  'rules' => [
    { 'pos' => %w[EC EF EP ETM ETN VCP XSA XSN XSV], 'cost' => 3000 },
    { 'pos' => %w[JC JKB JKC JKG JKO JKQ JKS JKV JX], 'cost' => 6000 }
  ]
})
```

> [!NOTE]
> v6.1.0 から、ko-dic では mecab-ko と同じく左側空白ペナルティがデフォルトで適用されます。たとえば `검색 이 잘 된다` の `이` は感動詞 `IC` と解析されます（ペナルティがなければ主格助詞 `JKS`）。オフにするには `builder.set_space_penalty(false)` を呼び出してください。`Lindera::Tokenizer.new(dictionary, ...)` で直接作成したトークナイザーは常にデフォルトを使うため、オフにするには `TokenizerBuilder` を使ってください。ルールを同梱しているのは Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースの ko-dic ではペナルティはオフのままで、辞書を再ビルドするまで `true` はエラーになります。詳しくは [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

#### `append_character_filter(kind, args)`

前処理パイプラインに文字フィルタを追加します。

```ruby
builder.append_character_filter('unicode_normalize', { 'kind' => 'nfkc' })
```

#### `append_token_filter(kind, args)`

後処理パイプラインにトークンフィルタを追加します。`args` が不要な場合は `nil` を渡します。

```ruby
builder.append_token_filter('lowercase', nil)
```

`set_space_penalty`、`append_character_filter`、`append_token_filter` の引数は、配列と hash の入れ子が 128 段までです。それより深い値や、自分自身を含む値を渡すと `ArgumentError` が発生します。

### ビルド

#### `build`

設定された内容で `Tokenizer` をビルドして返します。

```ruby
tokenizer = builder.build
```

## Tokenizer

`Lindera::Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `Lindera::Tokenizer.new(dictionary, mode = nil, user_dictionary = nil, space_penalty: nil)`

読み込み済みの辞書から直接トークナイザーを作成します。`mode` には、[`TokenizerBuilder#set_mode`](#set_modemode) と同じく [`Lindera::Mode`](#mode) を渡します。`nil`（デフォルト）は normal モードを表し、`Lindera::Mode` でない値（モード文字列 `'normal'` など）を渡すと `TypeError` が発生します。`user_dictionary` は省略できます。

キーワード引数 `space_penalty:` には [`TokenizerBuilder#set_space_penalty`](#set_space_penaltyvalue) と同じ値を渡せます。`nil`（デフォルト）では辞書が同梱する左側空白ペナルティのルールを使うため、ko-dic ではペナルティがオンになります。オフにするには `false` を渡します。不正な設定では、`set_space_penalty` や `build` と同じエラーが発生します。

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary)

# 左側空白ペナルティなしの ko-dic
ko_tokenizer = Lindera::Tokenizer.new(Lindera.load_dictionary('embedded://ko-dic'), space_penalty: false)
```

ユーザー辞書を使用する場合：

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
user_dict = Lindera.load_user_dictionary_for('/path/to/user_dictionary.csv', dictionary)
tokenizer = Lindera::Tokenizer.new(dictionary, Lindera::Mode.normal, user_dict)
```

### Tokenizer メソッド

#### `tokenize(text)`

入力テキストをトークナイズし、`Token` オブジェクトの配列を返します。

```ruby
tokens = tokenizer.tokenize('形態素解析')
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `String` | トークナイズするテキスト |

**戻り値:** `Array<Token>`

#### `tokenize_surfaces(text)`

入力テキストをトークナイズし、トークンの surface のみを文字列の配列として返します。分かち書き用途の高速パスです。`Token` オブジェクトを生成せず、形態素の詳細情報もロードしないため、surface 文字列だけが必要な場合は `tokenize` より大幅に高速です。結果は `tokenizer.tokenize(text).map(&:surface)` と一致します。

```ruby
surfaces = tokenizer.tokenize_surfaces('形態素解析')
# ["形態素", "解析"]
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `String` | トークナイズするテキスト |

**戻り値:** `Array<String>`

#### `tokenize_nbest(text, n, unique, cost_threshold)`

N-best トークナイズ結果を返します。各結果はトータルパスコストとペアになっています。

```ruby
results = tokenizer.tokenize_nbest('すもももももももものうち', 3, false, nil)
results.each do |tokens, cost|
  puts "#{cost}: #{tokens.map(&:surface).join(' / ')}"
end
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `String` | トークナイズするテキスト |
| `n` | `Integer` | 返す結果の数 |
| `unique` | `Boolean` | 結果の重複を排除（`false` で無効） |
| `cost_threshold` | `Integer` または `nil` | 最良パスからの最大コスト差（`nil` で無制限） |

**戻り値:** `Array<[Array<Token>, Integer]>`

## Mode

`Lindera::Mode` は、[`TokenizerBuilder#set_mode`](#set_modemode) と [`Tokenizer.new`](#linderatokenizernewdictionary-mode--nil-user_dictionary--nil-space_penalty-nil) が受け取るトークナイズモードです。Rust の `lindera::mode::Mode` に対応し、normal モードか、[`Lindera::Penalty`](#penalty) を持つ decompose モードのどちらかを表します。

### Mode の作成

`Lindera::Mode` には `new` がありません（`Lindera::Mode.new` は `TypeError` を発生させます）。次のクラスメソッドでモードを作成します。

#### `Lindera::Mode.normal`

normal モードを返します。

#### `Lindera::Mode.decompose(penalty = nil)`

`penalty`（`Lindera::Penalty`）を持つ decompose モードを返します。`nil` を渡すか引数を省略すると、既定のペナルティを使います。`Lindera::Penalty` でない値を渡すと `TypeError` が発生します。

#### `Lindera::Mode.from_str(name)`

`name` という名前のモードを、Rust の `Mode` と同じ規則で解析して返します。受け付ける名前は `'normal'` と `'decompose'` だけで、大文字と小文字を区別します。`'decompose'` からは既定のペナルティを持つ decompose モードができます。`'Normal'` などそれ以外の名前を渡すと、メッセージに `Invalid mode` を含む `RuntimeError` が発生します。設定などから実行時に読み込んだモード名の変換に使います。

```ruby
require 'lindera'

normal = Lindera::Mode.normal
decompose = Lindera::Mode.decompose
no_length_penalty = Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, 0))
from_setting = Lindera::Mode.from_str('decompose')
```

### Mode メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `name` | `String` | `"normal"` または `"decompose"` |
| `to_s` | `String` | `name` と同じ |
| `inspect` | `String` | 名前と、decompose モードではそのペナルティ |
| `penalty` | `Penalty` または `nil` | decompose モードのペナルティのコピー。normal モードでは `nil` |
| `normal?` | `Boolean` | normal モードの場合 `true` |
| `decompose?` | `Boolean` | decompose モードの場合 `true` |
| `==` | `Boolean` | 相手が名前とペナルティの等しい `Mode` の場合 `true` |

モードは変更できません。setter メソッドはなく、`penalty` はペナルティのコピーを返します。

`==` は名前とペナルティを比較し、文字列 `'normal'` のような別の型の値には `false` を返します。`eql?` と `hash` は同一性による比較のままです。Rust の `Mode` が `Eq` を実装し、`Hash` を実装していないためです。そのため、等しい 2 つのモードも `Hash` のキーとしては別物になり、`uniq` では両方が残ります。

```ruby
mode = Lindera::Mode.decompose(Lindera::Penalty.new(nil, 0, nil, 0))
mode.name                                   # "decompose"
mode.normal?                                # false
mode.decompose?                             # true
mode.penalty.kanji_penalty_length_penalty   # 0
mode.inspect
# "#<Lindera::Mode: decompose Penalty(kanji_threshold=2, kanji_penalty=0, other_threshold=7, other_penalty=0)>"

Lindera::Mode.normal.inspect   # "#<Lindera::Mode: normal>"
Lindera::Mode.normal.penalty   # nil

Lindera::Mode.decompose == Lindera::Mode.from_str('decompose')  # true
Lindera::Mode.decompose == mode                                 # false（ペナルティが異なる）
Lindera::Mode.normal == 'normal'                                # false
```

## Penalty

`Lindera::Penalty` は decompose モードのセグメンテーションで使用される、長さに基づくペナルティのしきい値を設定します。[`Lindera::Mode.decompose`](#linderamodedecomposepenalty--nil) に渡して使います。

### Penalty の作成

#### `Lindera::Penalty.new(kanji_penalty_length_threshold, kanji_penalty_length_penalty, other_penalty_length_threshold, other_penalty_length_penalty)`

4つの位置引数はすべて必須ですが、それぞれ `nil` を渡すとデフォルト値（下記参照）にフォールバックします。

```ruby
require 'lindera'

penalty = Lindera::Penalty.new(2, 3000, 7, 1700)
penalty = Lindera::Penalty.new(nil, nil, nil, nil)  # すべてデフォルト値を使用
```

### Penalty プロパティ

すべてのプロパティは読み取り専用です（setter メソッドはありません）：

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `Integer` | `2` | ペナルティが適用される漢字のみの表層形の長さのしきい値 |
| `kanji_penalty_length_penalty` | `Integer` | `3000` | しきい値を超える漢字のみの表層形に加算されるコストペナルティ |
| `other_penalty_length_threshold` | `Integer` | `7` | 漢字のみでない表層形にペナルティが適用される長さのしきい値 |
| `other_penalty_length_penalty` | `Integer` | `1700` | しきい値を超える漢字のみでない表層形に加算されるコストペナルティ |

```ruby
penalty = Lindera::Penalty.new(nil, nil, nil, nil)
penalty.kanji_penalty_length_threshold  # 2
```

### Penalty の使い方

`Penalty` は [`Lindera::Mode.decompose`](#linderamodedecomposepenalty--nil) に渡し、そのモードを [`TokenizerBuilder#set_mode`](#set_modemode) か [`Tokenizer.new`](#linderatokenizernewdictionary-mode--nil-user_dictionary--nil-space_penalty-nil) に渡します。長さのペナルティをなくすと、decompose モードでも長い語が分割されずに残ります。

```ruby
require 'lindera'

text = '関西国際空港限定トートバッグ'
penalty = Lindera::Penalty.new(nil, 0, nil, 0)

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('embedded://ipadic')
builder.set_mode(Lindera::Mode.decompose)
p builder.build.tokenize(text).map(&:surface)
# => ["関西", "国際", "空港", "限定", "トートバッグ"]

builder.set_mode(Lindera::Mode.decompose(penalty))
p builder.build.tokenize(text).map(&:surface)
# => ["関西国際空港", "限定", "トートバッグ"]

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary, Lindera::Mode.decompose(penalty))
p tokenizer.tokenize(text).map(&:surface)
# => ["関西国際空港", "限定", "トートバッグ"]
```

## Token

`Token` は単一の形態素トークンを表します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `String` | トークンの表層形 |
| `byte_start` | `Integer` | 元テキストでの開始バイト位置 |
| `byte_end` | `Integer` | 元テキストでの終了バイト位置 |
| `position` | `Integer` | トークンの位置インデックス |
| `word_id` | `Integer` | 辞書の単語 ID |
| `details` | `Array<String>` | 形態素の詳細情報（品詞、読みなど） |

さらに、述語メソッド `unknown?` は辞書に登録されていない単語の場合 `true` を返します：

```ruby
token.unknown?  # => false
```

### Token メソッド

#### `get_detail(index)`

指定されたインデックスの詳細文字列を返します。インデックスが最後の詳細より先の場合は `nil` を返します。負のインデックスは `RangeError` を投げます（[整数の引数](../lindera-ruby.md#整数の引数)を参照）。

```ruby
token = tokenizer.tokenize('東京')[0]
pos = token.get_detail(0)        # 例: "名詞"
subpos = token.get_detail(1)     # 例: "固有名詞"
reading = token.get_detail(7)    # 例: "トウキョウ"
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `index` | `Integer` | details 配列へのゼロベースインデックス |

**戻り値:** `String` または `nil`

#### `to_h` / `to_hash`

トークンを Symbol をキーとするプレーンな `Hash` として返します。各フィールドは
Ruby の自然な型を保つため、独自のエンコーダなしでシリアライズできます。
`to_h` と `to_hash` は同一のメソッドです。

```ruby
require 'json'

data = tokenizer.tokenize('東京')[0].to_h
# {surface: "東京", byte_start: 0, byte_end: 6, position: 0,
#  word_id: 12345, is_unknown: false, details: [...]}

JSON.generate(tokens.map(&:to_h))
```

**戻り値:** `:surface`、`:byte_start`、`:byte_end`、`:position`、`:word_id`、
`:is_unknown`、`:details` をキーに持つ `Hash`。

`details` の構造は辞書によって異なります：

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: UniDic 仕様に準拠した詳細な形態素情報
- **ko-dic / CC-CEDICT / Jieba**: 各辞書固有の詳細フォーマット

## Schema

`Lindera::Schema` はフィールド名の順序付きリストを保持し、フィールド名とインデックスの間の相互変換を提供します。`Metadata#dictionary_schema` と `Metadata#user_dictionary_schema` で使用されます（[辞書管理](./dictionary_management.md) を参照）。

### Schema の作成

#### `Lindera::Schema.new(fields)`

フィールド名の配列からスキーマを作成します。

```ruby
require 'lindera'

schema = Lindera::Schema.new(%w[
  surface
  left_context_id
  right_context_id
  cost
  major_pos
  reading
])
```

#### `Lindera::Schema.create_default`

組み込みのデフォルトスキーマを返します。IPADIC 形式に対応する13フィールド（`surface`、`left_context_id`、`right_context_id`、`cost`、`major_pos`、`pos_detail_1`、`pos_detail_2`、`pos_detail_3`、`conjugation_type`、`conjugation_form`、`base_form`、`reading`、`pronunciation`）です。

```ruby
schema = Lindera::Schema.create_default
```

### Schema メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `fields` | `Array<String>` | すべてのフィールド名（順序どおり） |
| `get_all_fields` | `Array<String>` | `fields` と同じ |
| `field_count` | `Integer` | フィールドの総数 |
| `get_field_index(name)` | `Integer` または `nil` | `name` という名前のフィールドのインデックス |
| `get_field_name(index)` | `String` または `nil` | `index` にあるフィールド名 |
| `get_custom_fields` | `Array<String>` | 固定の4フィールド（`surface`、`left_context_id`、`right_context_id`、`cost`）以降のフィールド名 |
| `get_field_by_name(name)` | `FieldDefinition` または `nil` | `name` の完全なフィールド定義 |
| `validate_record(record)` | `nil` | `record` がスキーマと一致しない場合 `ArgumentError` を発生 |
| `to_s` | `String` | 例: `"Schema(fields=13)"` |
| `inspect` | `String` | フィールドの全リスト |

```ruby
schema = Lindera::Schema.create_default

schema.field_count                  # 13
schema.get_field_index('cost')      # 3
schema.get_field_name(0)            # "surface"
schema.get_custom_fields            # ["major_pos", "pos_detail_1", ..., "pronunciation"]

field = schema.get_field_by_name('surface')
puts "#{field.index} #{field.name} #{field.field_type}"  # 0 surface surface

schema.validate_record([
  '東京', '1288', '1288', '100',
  '名詞', '固有名詞', '地域', '一般', '*', '*',
  '東京', 'トウキョウ', 'トーキョー'
])
```

## FieldDefinition

`Lindera::FieldDefinition` は `Schema` 内の単一フィールドを表します。インスタンスは `Schema#get_field_by_name` からのみ取得でき、公開コンストラクタはありません（`Lindera::FieldDefinition.new` は `TypeError` を発生させます）。

### FieldDefinition プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `index` | `Integer` | スキーマ内でのフィールドの位置（ゼロベース） |
| `name` | `String` | フィールド名 |
| `field_type` | `FieldType` | フィールドタイプ |
| `description` | `String` または `nil` | 説明（任意） |

```ruby
schema = Lindera::Schema.create_default
field = schema.get_field_by_name('surface')

field.index         # 0
field.name          # "surface"
field.field_type    # #<Lindera::FieldType: surface>
field.description    # nil（デフォルトスキーマでは説明は設定されていない）
```

## FieldType

`Lindera::FieldType` はフィールドの種別を列挙します。`FieldDefinition` と同様、インスタンスは `Schema`（`FieldDefinition#field_type` 経由）からのみ取得でき、公開コンストラクタはありません。

`to_s`（および `inspect`）は次のいずれかを返します：

- `"surface"` -- 表層形（単語のテキスト）
- `"left_context_id"` -- 左文脈 ID
- `"right_context_id"` -- 右文脈 ID
- `"cost"` -- 単語コスト
- `"custom"` -- その他の辞書固有フィールド

```ruby
field = Lindera::Schema.create_default.get_field_by_name('surface')
field.field_type.to_s  # "surface"
```

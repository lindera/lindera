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

トークナイズモードを設定します。

- `"normal"` -- 標準的なトークナイズ（デフォルト）
- `"decompose"` -- 複合語をより小さな単位に分解

```ruby
builder.set_mode('normal')
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
builder.set_user_dictionary('/path/to/user_dictionary')
```

#### `set_keep_whitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。

```ruby
builder.set_keep_whitespace(true)
```

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

### ビルド

#### `build`

設定された内容で `Tokenizer` をビルドして返します。

```ruby
tokenizer = builder.build
```

## Tokenizer

`Lindera::Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `Lindera::Tokenizer.new(dictionary, mode, user_dictionary)`

読み込み済みの辞書から直接トークナイザーを作成します。`user_dictionary` が不要な場合は `nil` を渡します。

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('embedded://ipadic')
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', nil)
```

ユーザー辞書を使用する場合：

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)
tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dict)
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

`Lindera::Mode` はトークナイズモードを表します。モードの確認・比較のためのスタンドアロンヘルパーとして提供されています。`TokenizerBuilder#set_mode` と `Tokenizer.new` は現在、`Mode` インスタンスではなく単純なモード文字列（`"normal"` または `"decompose"`）のみを受け付けます（下記の `Penalty` の制限事項も参照してください）。

### Mode の作成

#### `Lindera::Mode.new(mode_str)`

`Mode` を作成します。引数は必須ですが `nil` を渡すこともできます。`"normal"` / `"Normal"`（`mode_str` が `nil` の場合に使用）または `"decompose"` / `"Decompose"` を受け付けます。それ以外の値を渡すと `ArgumentError` が発生します。

```ruby
require 'lindera'

mode = Lindera::Mode.new('normal')
mode = Lindera::Mode.new('decompose')
mode = Lindera::Mode.new(nil)  # デフォルトの "normal" になる
```

### Mode メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `to_s` | `String` | `"normal"` または `"decompose"` |
| `name` | `String` | `to_s` と同じ |
| `inspect` | `String` | 例: `"#<Lindera::Mode: decompose>"` |
| `normal?` | `Boolean` | モードが `"normal"` の場合 `true` |
| `decompose?` | `Boolean` | モードが `"decompose"` の場合 `true` |

```ruby
mode = Lindera::Mode.new('decompose')
mode.to_s        # "decompose"
mode.normal?      # false
mode.decompose?   # true
```

## Penalty

`Lindera::Penalty` は `"decompose"` モードのセグメンテーションで使用される、長さに基づくペナルティのしきい値を設定します。

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

**現在の制限:** 現時点では `Penalty` を `Tokenizer` や `TokenizerBuilder` に渡す方法はありません。`set_mode` と `Tokenizer.new` は単純なモード文字列のみを受け付け、内部的に `"decompose"` モードは常に `Penalty` のデフォルト値を使用します -- カスタムの `Penalty` インスタンスを作成しても、現時点ではトークナイズには反映されません。

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

指定されたインデックスの詳細文字列を返します。インデックスが範囲外の場合は `nil` を返します。

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

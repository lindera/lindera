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
| `is_unknown` | `Boolean` | 辞書に登録されていない単語の場合 `true` |
| `details` | `Array<String>` または `nil` | 形態素の詳細情報（品詞、読みなど） |

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

# 辞書管理

Lindera Ruby は、形態素解析で使用する辞書の読み込み、ビルド、管理のためのメソッドを提供します。

## 辞書の読み込み

### システム辞書

`Lindera.load_dictionary(uri)` を使用してシステム辞書を読み込みます。[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済み辞書をダウンロードし、展開したディレクトリのパスを指定してください：

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
```

**埋め込み辞書（上級者向け）** -- `embed-*` feature フラグ付きでビルドした場合、埋め込み辞書を使用できます：

```ruby
dictionary = Lindera.load_dictionary('embedded://ipadic')
```

### ユーザー辞書

ユーザー辞書はシステム辞書にカスタム語彙を追加します。

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)
```

トークナイザーの作成時にユーザー辞書を渡します：

```ruby
require 'lindera'

dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
user_dict = Lindera.load_user_dictionary('/path/to/user_dictionary', metadata)

tokenizer = Lindera::Tokenizer.new(dictionary, 'normal', user_dict)
```

または、ビルダー経由で設定します：

```ruby
require 'lindera'

builder = Lindera::TokenizerBuilder.new
builder.set_dictionary('/path/to/ipadic')
builder.set_user_dictionary('/path/to/user_dictionary')
tokenizer = builder.build
```

## 辞書のビルド

### システム辞書のビルド

ソースファイルからシステム辞書をビルドします：

```ruby
require 'lindera'

metadata = Lindera::Metadata.from_json_file('metadata.json')
Lindera.build_dictionary('/path/to/input_dir', '/path/to/output_dir', metadata)
```

入力ディレクトリには辞書のソースファイル（CSV レキシコン、matrix.def など）が含まれている必要があります。

### ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします：

```ruby
require 'lindera'

metadata = Lindera::Metadata.from_json_file('metadata.json')
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/output_dir', metadata)
```

`metadata` パラメータは省略可能です。省略した場合はデフォルトのメタデータ値が使用されます：

```ruby
Lindera.build_user_dictionary('ipadic', 'user_words.csv', '/path/to/output_dir', nil)
```

> [!NOTE]
> 第一引数（`kind`、上の例では `'ipadic'`）は現時点では未使用です -- 将来の利用のために予約
> されているだけで、ビルドには影響しません。現時点では任意の文字列を渡すことができます。

## Metadata

`Lindera::Metadata` クラスは辞書のパラメータを設定します。

### Metadata の作成

```ruby
require 'lindera'

# 標準設定でデフォルトのメタデータを作成
metadata = Lindera::Metadata.create_default
```

`Lindera::Metadata.new` は9つのプロパティすべてを必須の位置引数として受け取ります
（それぞれ `nil` を渡すとデフォルト値にフォールバックします）。特定の値を上書きしたい
場合にのみ使用してください：

```ruby
metadata = Lindera::Metadata.new(
  'my_dict', # name
  'UTF-8',   # encoding
  -10_000,   # default_word_cost
  1288,      # default_left_context_id
  1288,      # default_right_context_id
  '*',       # default_field_value
  false,     # flexible_csv
  false,     # skip_invalid_cost_or_id
  false      # normalize_details
)
```

### JSON ファイルからの読み込み

```ruby
metadata = Lindera::Metadata.from_json_file('metadata.json')
```

### 辞書からのメタデータ取得

読み込み済みの辞書からメタデータを取得できます：

```ruby
dictionary = Lindera.load_dictionary('/path/to/ipadic')
metadata = dictionary.metadata
```

### プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `name` | `String` | `"default"` | 辞書名 |
| `encoding` | `String` | `"UTF-8"` | 文字エンコーディング |
| `default_word_cost` | `Integer` | `-10000` | 未知語のデフォルトコスト |
| `default_left_context_id` | `Integer` | `1288` | デフォルトの左文脈 ID |
| `default_right_context_id` | `Integer` | `1288` | デフォルトの右文脈 ID |
| `default_field_value` | `String` | `"*"` | 欠損フィールドのデフォルト値 |
| `flexible_csv` | `Boolean` | `false` | 柔軟な CSV パースを許可 |
| `skip_invalid_cost_or_id` | `Boolean` | `false` | 無効なコストまたは ID のエントリーをスキップ |
| `normalize_details` | `Boolean` | `false` | 形態素の詳細情報を正規化 |

# 辞書管理

Lindera Python は、形態素解析で使用する辞書の読み込み、ビルド、管理のための関数を提供します。

## 辞書の読み込み

### システム辞書

`load_dictionary(uri)` を使用してシステム辞書を読み込みます。[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済み辞書をダウンロードし、展開したディレクトリのパスを指定してください：

```python
from lindera import load_dictionary

dictionary = load_dictionary("/path/to/ipadic")
```

**埋め込み辞書（上級者向け）** -- `embed-*` feature フラグ付きでビルドした場合、埋め込み辞書を使用できます：

```python
dictionary = load_dictionary("embedded://ipadic")
```

読み込んだ `Dictionary` は、自身のメタデータも公開しています：

```python
print(dictionary.metadata_name())      # 例: "ipadic"
print(dictionary.metadata_encoding())  # 例: "UTF-8"
metadata = dictionary.metadata()       # Metadata オブジェクト全体
```

これは、システム辞書と同じメタデータ（スキーマ、エンコーディングなど）を
共有する必要があるユーザー辞書を読み込む際に便利です。
[`lindera-python/examples/tokenize_with_userdict.py`](https://github.com/lindera/lindera/blob/main/lindera-python/examples/tokenize_with_userdict.py)
を参照してください：

```python
from lindera import Tokenizer, load_dictionary, load_user_dictionary

dictionary = load_dictionary("embedded://ipadic")
metadata = dictionary.metadata()
user_dictionary = load_user_dictionary("/path/to/user_dictionary.csv", metadata)

tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dictionary)
```

### ユーザー辞書

ユーザー辞書はシステム辞書にカスタム語彙を追加します。

```python
from lindera import load_user_dictionary, Metadata

metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary", metadata)
```

トークナイザーのビルド時にユーザー辞書を渡します：

```python
from lindera import Tokenizer, load_dictionary, load_user_dictionary, Metadata

dictionary = load_dictionary("/path/to/ipadic")
metadata = Metadata()
user_dict = load_user_dictionary("/path/to/user_dictionary", metadata)

tokenizer = Tokenizer(dictionary, mode="normal", user_dictionary=user_dict)
```

または、ビルダー経由で設定します：

```python
from lindera import TokenizerBuilder

tokenizer = (
    TokenizerBuilder()
    .set_dictionary("/path/to/ipadic")
    .set_user_dictionary("/path/to/user_dictionary")
    .build()
)
```

## 辞書のビルド

### システム辞書のビルド

ソースファイルからシステム辞書をビルドします：

```python
from lindera import build_dictionary, Metadata

metadata = Metadata(name="custom", encoding="UTF-8")
build_dictionary("/path/to/input_dir", "/path/to/output_dir", metadata)
```

入力ディレクトリには辞書のソースファイル（CSV レキシコン、matrix.def など）が含まれている必要があります。

### ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします：

```python
from lindera import build_user_dictionary, Metadata

metadata = Metadata()
build_user_dictionary("ipadic", "user_words.csv", "/path/to/output_dir", metadata)
```

`metadata` パラメータは省略可能です。省略した場合はデフォルトのメタデータ値が使用されます：

```python
build_user_dictionary("ipadic", "user_words.csv", "/path/to/output_dir")
```

**注意:** 最初の引数（上記の `"ipadic"`）は現在未使用で、将来のために予約されて
いるものです -- ビルドの選択や設定には一切影響しません。ビルドの挙動は
`metadata`（特に `metadata.user_dictionary_schema`）によってのみ制御されます。
現時点では任意の文字列を渡すことができます。

## Metadata

`Metadata` クラスは辞書のパラメータを設定します。

### Metadata の作成

```python
from lindera import Metadata

# デフォルトのメタデータ
metadata = Metadata()

# カスタムメタデータ
metadata = Metadata(
    name="my_dictionary",
    encoding="UTF-8",
    default_word_cost=-10000,
)
```

### JSON からの読み込み

```python
metadata = Metadata.from_json_file("metadata.json")
```

### プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `name` | `str` | `"default"` | 辞書名 |
| `encoding` | `str` | `"UTF-8"` | 文字エンコーディング |
| `default_word_cost` | `int` | `-10000` | 未知語のデフォルトコスト |
| `default_left_context_id` | `int` | `1288` | デフォルトの左文脈 ID |
| `default_right_context_id` | `int` | `1288` | デフォルトの右文脈 ID |
| `default_field_value` | `str` | `"*"` | 欠損フィールドのデフォルト値 |
| `flexible_csv` | `bool` | `False` | 柔軟な CSV パースを許可 |
| `skip_invalid_cost_or_id` | `bool` | `False` | 無効なコストまたは ID のエントリーをスキップ |
| `normalize_details` | `bool` | `False` | 形態素の詳細情報を正規化 |
| `dictionary_schema` | `Schema` | IPADIC スキーマ | メイン辞書のスキーマ |
| `user_dictionary_schema` | `Schema` | 最小スキーマ | ユーザー辞書のスキーマ |

すべてのプロパティは取得と設定の両方をサポートしています：

```python
metadata = Metadata()
metadata.name = "custom_dict"
metadata.encoding = "EUC-JP"
print(metadata.name)  # "custom_dict"
```

### `to_dict()`

メタデータの辞書表現を返します：

```python
metadata = Metadata(name="test")
print(metadata.to_dict())
```

## Schema

`Schema`、`FieldDefinition`、`FieldType` は、辞書エントリーのフィールド構成を
表します。スキーマは `Metadata.dictionary_schema` と
`Metadata.user_dictionary_schema`（上記の表を参照）で使用されます。

### FieldType

`FieldType` は単一フィールドの種別を列挙します：

- `FieldType.Surface` -- 表層形（単語のテキスト）
- `FieldType.LeftContextId` -- 左文脈 ID
- `FieldType.RightContextId` -- 右文脈 ID
- `FieldType.Cost` -- 単語コスト
- `FieldType.Custom` -- その他の辞書固有フィールド

### FieldDefinition

`FieldDefinition` はスキーマ内の単一フィールドを表します。

#### `FieldDefinition(index, name, field_type, description=None)`

```python
from lindera import FieldDefinition, FieldType

field = FieldDefinition(0, "surface", FieldType.Surface, "Surface form")
```

**プロパティ**（読み取り専用）：

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `index` | `int` | スキーマ内でのフィールドの位置（0 始まり） |
| `name` | `str` | フィールド名 |
| `field_type` | `FieldType` | フィールドの種別 |
| `description` | `str` または `None` | 任意の説明文 |

### Schema の作成

`Schema` はフィールド名の順序付きリストを保持し、フィールド名とインデックス間の
相互参照を提供します。

#### `Schema(fields)`

フィールド名のリストからスキーマを作成します。

```python
from lindera import Schema

schema = Schema([
    "surface",
    "left_context_id",
    "right_context_id",
    "cost",
    "major_pos",
    "reading",
])
```

#### `Schema.create_default()`

組み込みのデフォルトスキーマを返す静的メソッドです。13 個のフィールドから成り、
IPADIC 形式のレイアウトに対応します（`surface`, `left_context_id`,
`right_context_id`, `cost`, `major_pos`, `pos_detail_1`, `pos_detail_2`,
`pos_detail_3`, `conjugation_type`, `conjugation_form`, `base_form`, `reading`,
`pronunciation`）。

```python
schema = Schema.create_default()
```

### Schema のメソッドとプロパティ

| メンバー | 戻り値 | 説明 |
| --- | --- | --- |
| `fields`（プロパティ） | `list[str]` | すべてのフィールド名（順序どおり） |
| `field_count()` | `int` | フィールドの総数 |
| `get_field_index(name)` | `int` または `None` | `name` という名前のフィールドのインデックス |
| `get_field_name(index)` | `str` または `None` | `index` のフィールド名 |
| `get_custom_fields()` | `list[str]` | 4 つの固定フィールド（`surface`, `left_context_id`, `right_context_id`, `cost`）以降のフィールド名 |
| `get_field_by_name(name)` | `FieldDefinition` または `None` | `name` の完全なフィールド定義 |
| `validate_record(record)` | `None` | `record` がスキーマと一致しない場合 `ValueError` を送出 |
| `__len__()` | `int` | `field_count()` と同じ |

```python
schema = Schema.create_default()

schema.field_count()               # 13
schema.get_field_index("cost")     # 3
schema.get_field_name(0)           # "surface"
schema.get_custom_fields()         # ["major_pos", "pos_detail_1", ..., "pronunciation"]
len(schema)                        # 13

field = schema.get_field_by_name("surface")
print(field.index, field.name, field.field_type)  # 0 surface FieldType.Surface

schema.validate_record([
    "東京", "1288", "1288", "100",
    "名詞", "固有名詞", "地域", "一般", "*", "*",
    "東京", "トウキョウ", "トーキョー",
])
```

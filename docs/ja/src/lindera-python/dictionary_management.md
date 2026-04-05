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


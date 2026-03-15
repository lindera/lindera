# Tokenizer API

## TokenizerBuilder

`TokenizerBuilder` はビルダーパターンを使用して `Tokenizer` インスタンスを設定・構築します。

### コンストラクタ

#### `TokenizerBuilder()`

デフォルト設定で新しいビルダーを作成します。

```python
from lindera import TokenizerBuilder

builder = TokenizerBuilder()
```

#### `TokenizerBuilder().from_file(file_path)`

JSON ファイルから設定を読み込み、新しいビルダーを返します。

```python
builder = TokenizerBuilder().from_file("config.json")
```

### 設定メソッド

すべてのセッターメソッドはメソッドチェーンのために `self` を返します。

#### `set_mode(mode)`

トークナイズモードを設定します。

- `"normal"` -- 標準的なトークナイズ（デフォルト）
- `"decompose"` -- 複合語をより小さな単位に分解

```python
builder.set_mode("normal")
```

#### `set_dictionary(path)`

システム辞書のパスまたは URI を設定します。

```python
# 埋め込み辞書を使用
builder.set_dictionary("embedded://ipadic")

# 外部辞書を使用
builder.set_dictionary("/path/to/dictionary")
```

#### `set_user_dictionary(uri)`

ユーザー辞書の URI を設定します。

```python
builder.set_user_dictionary("/path/to/user_dictionary")
```

#### `set_keep_whitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。

```python
builder.set_keep_whitespace(True)
```

#### `append_character_filter(kind, args=None)`

前処理パイプラインに文字フィルタを追加します。

```python
builder.append_character_filter("unicode_normalize", {"kind": "nfkc"})
```

#### `append_token_filter(kind, args=None)`

後処理パイプラインにトークンフィルタを追加します。

```python
builder.append_token_filter("lowercase", {})
```

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```python
tokenizer = builder.build()
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `Tokenizer(dictionary, mode="normal", user_dictionary=None)`

読み込み済みの辞書から直接トークナイザーを作成します。

```python
from lindera import Tokenizer, load_dictionary

dictionary = load_dictionary("embedded://ipadic")
tokenizer = Tokenizer(dictionary, mode="normal")
```

### Tokenizer メソッド

#### `tokenize(text)`

入力テキストをトークナイズし、`Token` オブジェクトのリストを返します。

```python
tokens = tokenizer.tokenize("形態素解析")
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `str` | トークナイズするテキスト |

**戻り値:** `list[Token]`

#### `tokenize_nbest(text, n, unique=False, cost_threshold=None)`

N-best トークナイズ結果を返します。各結果はトータルパスコストとペアになっています。

```python
results = tokenizer.tokenize_nbest("すもももももももものうち", n=3)
for tokens, cost in results:
    print(cost, [t.surface for t in tokens])
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `str` | トークナイズするテキスト |
| `n` | `int` | 返す結果の数 |
| `unique` | `bool` | 結果の重複を排除（デフォルト: `False`） |
| `cost_threshold` | `int` または `None` | 最良パスからの最大コスト差（デフォルト: `None`） |

**戻り値:** `list[tuple[list[Token], int]]`

## Token

`Token` は単一の形態素トークンを表します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `str` | トークンの表層形 |
| `byte_start` | `int` | 元テキストでの開始バイト位置 |
| `byte_end` | `int` | 元テキストでの終了バイト位置 |
| `position` | `int` | トークンの位置インデックス |
| `word_id` | `int` | 辞書の単語 ID |
| `is_unknown` | `bool` | 辞書に登録されていない単語の場合 `True` |
| `details` | `list[str]` または `None` | 形態素の詳細情報（品詞、読みなど） |

### Token メソッド

#### `get_detail(index)`

指定されたインデックスの詳細文字列を返します。インデックスが範囲外の場合は `None` を返します。

```python
token = tokenizer.tokenize("東京")[0]
pos = token.get_detail(0)        # 例: "名詞"
subpos = token.get_detail(1)     # 例: "固有名詞"
reading = token.get_detail(7)    # 例: "トウキョウ"
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `index` | `int` | details リストへのゼロベースインデックス |

**戻り値:** `str` または `None`

`details` の構造は辞書によって異なります：

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: UniDic 仕様に準拠した詳細な形態素情報
- **ko-dic / CC-CEDICT / Jieba**: 各辞書固有の詳細フォーマット

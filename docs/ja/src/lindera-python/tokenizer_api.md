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

YAML ファイルから設定を読み込み、新しいビルダーを返します。`segmenter`、
`character_filters`、`token_filters` を含む完全な例は
[`lindera-python/resources/lindera.yml`](https://github.com/lindera/lindera/blob/main/lindera-python/resources/lindera.yml)
を参照してください。

```python
builder = TokenizerBuilder().from_file("lindera.yml")
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

## Mode

`Mode` はトークナイズモードを表します。モードの確認や比較のためのスタンドアロンな
ヘルパーとして提供されています。`TokenizerBuilder.set_mode()` と `Tokenizer` の
コンストラクタは、現在は `Mode` インスタンスではなく単純なモード文字列
（`"normal"` または `"decompose"`）のみを受け付けます（下記 `Penalty` の制限事項
も参照）。

### Mode の作成

#### `Mode(mode_str=None)`

`Mode` を作成します。`"normal"` / `"Normal"`（省略時のデフォルト）または
`"decompose"` / `"Decompose"` を受け付けます。それ以外の値を指定すると
`ValueError` が発生します。

```python
from lindera import Mode

mode = Mode("normal")
mode = Mode("decompose")
mode = Mode()  # デフォルトは "normal"
```

### メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `__str__()` | `str` | `"normal"` または `"decompose"` |
| `__repr__()` | `str` | 例: `"Mode.Normal"` |
| `is_normal()` | `bool` | モードが `"normal"` の場合 `True` |
| `is_decompose()` | `bool` | モードが `"decompose"` の場合 `True` |

```python
mode = Mode("decompose")
str(mode)            # "decompose"
repr(mode)           # "Mode.Decompose"
mode.is_normal()      # False
mode.is_decompose()   # True
```

## Penalty

`Penalty` は `"decompose"` モードのセグメンテーションで使用される、長さに基づく
ペナルティの閾値を設定します。

### Penalty の作成

#### `Penalty(kanji_penalty_length_threshold=2, kanji_penalty_length_penalty=3000, other_penalty_length_threshold=7, other_penalty_length_penalty=1700)`

すべての引数は省略可能で、上記の値がデフォルトとして使用されます。

```python
from lindera import Penalty

penalty = Penalty(
    kanji_penalty_length_threshold=2,
    kanji_penalty_length_penalty=3000,
    other_penalty_length_threshold=7,
    other_penalty_length_penalty=1700,
)
```

### Penalty のプロパティ

4 つのフィールドはすべて取得・設定の両方をサポートしています：

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `int` | `2` | ペナルティが適用される、漢字のみの表層形の長さの閾値 |
| `kanji_penalty_length_penalty` | `int` | `3000` | 閾値を超える漢字のみの表層形に加算されるコストペナルティ |
| `other_penalty_length_threshold` | `int` | `7` | 漢字のみでない表層形にペナルティが適用される長さの閾値 |
| `other_penalty_length_penalty` | `int` | `1700` | 閾値を超える漢字のみでない表層形に加算されるコストペナルティ |

```python
penalty.kanji_penalty_length_threshold = 3
print(penalty.kanji_penalty_length_threshold)  # 3
```

**現在の制限事項:** 現時点では `Penalty` を `Tokenizer` や `TokenizerBuilder` に
渡す方法はありません。`set_mode()` と `Tokenizer` のコンストラクタは単純な
モード文字列のみを受け付け、内部的に `"decompose"` モードは常に `Penalty` の
デフォルト値を使用します -- カスタムの `Penalty` インスタンスを作成しても、
トークナイズには反映されません。

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

## エラーハンドリング

Lindera Python の関数は、カスタムの例外型ではなく標準的な Python の例外を送出します：

- `IOError`（`OSError` のエイリアス） -- ファイルが存在しない、読み込めないなどの
  I/O 関連の失敗
- `ValueError` -- それ以外のすべてのケース（不正な設定、パースエラー、
  トークナイズの失敗など）

```python
from lindera import load_dictionary

try:
    dictionary = load_dictionary("/path/that/does/not/exist")
except ValueError as e:
    print(f"Failed to load dictionary: {e}")
```

`lindera.LinderaError` クラスも登録されていますが、このクレート内のどの関数からも
送出されることはありません -- 手動で構築・送出する場合にのみ使用できます。この
ライブラリのエラーを処理する際は、`LinderaError` ではなく `IOError` /
`ValueError`（または一般的な `Exception`）をキャッチしてください。

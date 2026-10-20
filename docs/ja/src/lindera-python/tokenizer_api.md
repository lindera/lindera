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

トークナイズモードを設定します。`mode` は [`Mode`](#mode) です：

- `Mode.normal()` -- 標準的なトークナイズ（デフォルト）
- `Mode.decompose(penalty=None)` -- [`Penalty`](#penalty) の長さのペナルティを使い、複合語をより小さな単位に分解。`None` では既定のペナルティを使います

文字列 `"normal"` など `Mode` でない値を渡すと `TypeError` が送出されます。設定値のようにモードを名前で指定するときは、[`Mode.from_str()`](#modefrom_strname) で `Mode` を作成します。

```python
from lindera import Mode, Penalty

builder.set_mode(Mode.normal())
builder.set_mode(Mode.decompose(Penalty(kanji_penalty_length_penalty=0)))
builder.set_mode(Mode.from_str("decompose"))
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
builder.set_user_dictionary("/path/to/user_dictionary.csv")
```

#### `set_keep_whitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。残した空白はラティスにも残るため、他のトークンも、MeCab と同様に空白をラティス上で読み飛ばすデフォルトの出力と変わることがあります。空白を出力せずにラティスにだけ残すには、代わりに [`set_skip_whitespace(False)`](#set_skip_whitespaceskip) を使います。

```python
builder.set_keep_whitespace(True)
```

#### `set_skip_whitespace(skip)`

空白をラティス上で読み飛ばすかどうかを設定します。`skip` の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.skip_whitespace` と同じです：

- `True` -- 空白を読み飛ばし、MeCab と同様に空白の前後の語を直接接続します。
- `False` -- 空白を独立したノード（空白の見出し語か `SPACE` の未知語）としてラティスに残します（v7 より前の動作）。空白は引き続き出力から除外されます。
- `None` -- 辞書の `metadata.json` のデフォルトです。SudachiDict 以外では読み飛ばします。以前の呼び出しを取り消すときに使います。

ここで設定した値は、辞書のデフォルトと、`from_file` で読み込んだ `skip_whitespace` より優先されます。空白トークンを残すとき（`set_keep_whitespace(True)`）は効果がありません。残した空白は常にラティスに残るためです。それ以外の値では `TypeError` が送出されます。

たとえば IPADIC では、`Google が 新しい` の `が` は、デフォルトでは助詞（`助詞`）、`set_skip_whitespace(False)` では接続詞（`接続詞`）と解析されます。詳しくは [Segmenter](../lindera/segmenter.md#空白文字の扱い) を参照してください。

```python
# v7 より前と同じく、空白をラティスに残す
builder.set_skip_whitespace(False)
```

#### `set_space_penalty(value)`

韓国語向けの左側空白ペナルティ（left-space penalty）を設定します。直前に空白がある候補のうち、本来は前の語に空白なしで付く品詞（助詞、語尾など）の候補にコストを加算します。`value` の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.space_penalty` と同じです：

- `None` -- デフォルト。辞書が `metadata.json` に同梱するルールがあればそれを使います（ko-dic は mecab-ko-dic のルールを同梱し、他の同梱辞書はルールを持ちません）。以前の呼び出しを取り消すときにも使います。
- `False` -- ペナルティをオフにします。
- `True` -- 辞書のルールを要求します。ルールを同梱しない辞書では `build()` が `ValueError` を送出します。
- `{"rules": [{"pos": [...], "cost": n}, ...]}` 形式の `dict` -- 辞書のルールの代わりにこのルールを適用します。先頭品詞タグが `pos` に含まれる候補に `cost` を加算し、最初に一致したルールが使われます。

それ以外の値では `ValueError` が送出されます（任意のオブジェクトなど JSON に対応しない値は、`append_token_filter` と同様に `TypeError` になります）。

```python
# ペナルティをオフにする
builder.set_space_penalty(False)

# 明示的なルール
builder.set_space_penalty({
    "rules": [
        {"pos": ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], "cost": 3000},
        {"pos": ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], "cost": 6000},
    ]
})
```

> [!NOTE]
> v6.1.0 から、ko-dic では mecab-ko と同じく左側空白ペナルティがデフォルトで適用されます。たとえば `검색 이 잘 된다` の `이` は感動詞 `IC` と解析されます（ペナルティがなければ主格助詞 `JKS`）。オフにするには `builder.set_space_penalty(False)` を呼び出してください。ルールを同梱しているのは Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースの ko-dic ではペナルティはオフのままで、辞書を再ビルドするまで `True` はエラーになります。詳しくは [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

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

`set_space_penalty`、`append_character_filter`、`append_token_filter` の引数は、list と dict の入れ子が 128 段までです。それより深い値や、自分自身を含む値を渡すと `ValueError` が送出されます。

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```python
tokenizer = builder.build()
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `Tokenizer(dictionary, mode=None, user_dictionary=None, space_penalty=None)`

読み込み済みの辞書から直接トークナイザーを作成します。

`mode` には、[`TokenizerBuilder.set_mode`](#set_modemode) と同じく [`Mode`](#mode) を渡します。`None`（デフォルト）は normal モードです。文字列 `"normal"` などそれ以外の値を渡すと `TypeError` が送出されます。

`space_penalty` には [`TokenizerBuilder.set_space_penalty`](#set_space_penaltyvalue) と同じ値を渡せます。`None`（デフォルト）では辞書が同梱する左側空白ペナルティのルールを使うため、ko-dic ではペナルティがオンになります。オフにするには `False` を渡します。不正な設定では、`set_space_penalty` や `build()` と同じエラーが送出されます。

```python
from lindera import Mode, Tokenizer, load_dictionary

dictionary = load_dictionary("embedded://ipadic")
tokenizer = Tokenizer(dictionary, mode=Mode.normal())

# 左側空白ペナルティなしの ko-dic
ko_tokenizer = Tokenizer(load_dictionary("embedded://ko-dic"), space_penalty=False)
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

#### `tokenize_surfaces(text)`

入力テキストをトークナイズし、トークンの surface のみを文字列のリストとして返します。分かち書き用途の高速パスです。`Token` オブジェクトを生成せず、形態素の詳細情報もロードしないため、surface 文字列だけが必要な場合は `tokenize` より大幅に高速です。結果は `[t.surface for t in tokenizer.tokenize(text)]` と一致します。

```python
surfaces = tokenizer.tokenize_surfaces("形態素解析")
# ["形態素", "解析"]
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `str` | トークナイズするテキスト |

**戻り値:** `list[str]`

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

`Mode` は、[`TokenizerBuilder.set_mode()`](#set_modemode) と [`Tokenizer` のコンストラクタ](#tokenizerdictionary-modenone-user_dictionarynone-space_penaltynone) が受け取るトークナイズモードです。Rust の `lindera::mode::Mode` に対応し、normal モードか、[`Penalty`](#penalty) を持つ decompose モードのどちらかを表します。

### Mode の作成

`Mode` にはコンストラクタがないため、`Mode()` は `TypeError` を送出します。`Mode.Normal` や `Mode.Decompose` という属性もありません。モードは次のスタティックメソッドで作成します。

#### `Mode.normal()`

normal モード（標準的なトークナイズ）を返します。

#### `Mode.decompose(penalty=None)`

decompose モードを返します。decompose モードは長い語のコストに `penalty` の長さのペナルティを加算し、複合語をより小さな単位に分解します。`None`（デフォルト）では既定のペナルティ `Penalty()` を使います。`Penalty` でない値を渡すと `TypeError` が送出されます。

#### `Mode.from_str(name)`

Rust の `Mode` と同じ規則で名前を解釈し、`name` という名前のモードを返します。`"normal"` は normal モード、`"decompose"` は既定のペナルティを持つ decompose モードです。設定値のように実行時に読み込むモード名に使います。名前は大文字と小文字を区別し、`"Normal"` などそれ以外の名前ではメッセージに `Invalid mode` を含む `ValueError` が送出されます。`str` でない値を渡すと `TypeError` が送出されます。

```python
from lindera import Mode, Penalty

normal = Mode.normal()
decompose = Mode.decompose()
whole_words = Mode.decompose(Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0))
from_name = Mode.from_str("decompose")  # Mode.decompose() と等しい
```

### Mode のプロパティ

どちらのプロパティも読み取り専用です：

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `name` | `str` | `"normal"` または `"decompose"` |
| `penalty` | `Penalty` または `None` | decompose モードのペナルティのコピー。normal モードでは `None` |

### メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `__str__()` | `str` | モードの名前（`"normal"` または `"decompose"`） |
| `__repr__()` | `str` | `"Mode.normal()"`、またはペナルティを含む `Mode.decompose(...)` |
| `__eq__(other)` | `bool` | `other` が名前とペナルティの等しい `Mode` の場合 `True` |
| `is_normal()` | `bool` | normal モードの場合 `True` |
| `is_decompose()` | `bool` | decompose モードの場合 `True` |

```python
mode = Mode.decompose()
mode.name              # 'decompose'
mode.penalty           # Penalty(kanji_threshold=2, kanji_penalty=3000, other_threshold=7, other_penalty=1700)
str(mode)              # 'decompose'
repr(mode)             # 'Mode.decompose(Penalty(kanji_threshold=2, kanji_penalty=3000, other_threshold=7, other_penalty=1700))'
mode.is_normal()       # False
mode.is_decompose()    # True
Mode.normal().penalty  # None
repr(Mode.normal())    # 'Mode.normal()'
```

2 つのモードは、Rust の `Mode` と同じく、名前とペナルティが等しいときに等しくなります。文字列 `"normal"` など、ほかの型の値とは等しくなりません：

```python
Mode.from_str("decompose") == Mode.decompose()                               # True
Mode.decompose() == Mode.decompose(Penalty(kanji_penalty_length_penalty=0))  # False
Mode.normal() == "normal"                                                    # False
```

`Mode` は変更できません。プロパティに代入すると `AttributeError` が送出されます。`penalty` はコピーを返すため、返された `Penalty` を変更してもモードは変わりません。`Mode.decompose()` も、受け取った `Penalty` のコピーを保持します。また、Rust の `Mode` が `Eq` を実装し `Hash` を実装しないのと同じく、`Mode` はハッシュ化できません。`hash(mode)` は `TypeError` を送出するため、`Mode` を `dict` のキーや `set` の要素にはできません。

```python
mode = Mode.decompose()
penalty = mode.penalty
penalty.kanji_penalty_length_penalty = 0
mode.penalty.kanji_penalty_length_penalty  # 3000
```

## Penalty

`Penalty` は decompose モードのセグメンテーションで使用される、長さに基づく
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

### Penalty の使い方

`Penalty` は [`Mode.decompose()`](#modedecomposepenaltynone) に渡し、そのモードを [`TokenizerBuilder.set_mode()`](#set_modemode) か [`Tokenizer` のコンストラクタ](#tokenizerdictionary-modenone-user_dictionarynone-space_penaltynone) に渡します。長さのペナルティをなくすと、decompose モードでも長い語が分割されずに残ります。

```python
from lindera import Mode, Penalty, Tokenizer, TokenizerBuilder, load_dictionary

text = "関西国際空港限定トートバッグ"
penalty = Penalty(kanji_penalty_length_penalty=0, other_penalty_length_penalty=0)

builder = TokenizerBuilder().set_dictionary("embedded://ipadic")
tokenizer = builder.set_mode(Mode.decompose()).build()
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西', '国際', '空港', '限定', 'トートバッグ']

tokenizer = builder.set_mode(Mode.decompose(penalty)).build()
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西国際空港', '限定', 'トートバッグ']

tokenizer = Tokenizer(load_dictionary("embedded://ipadic"), mode=Mode.decompose(penalty))
print([token.surface for token in tokenizer.tokenize(text)])
# ['関西国際空港', '限定', 'トートバッグ']
```

normal モードはペナルティを持ちません。`Mode.normal()` は引数を取りません。

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

#### `to_dict()`

トークンをプレーンな `dict` として返します。各フィールドは Python の自然な型を
保つため、独自のエンコーダなしでシリアライズできます。

```python
import json

data = tokenizer.tokenize("東京")[0].to_dict()
# {'surface': '東京', 'byte_start': 0, 'byte_end': 6, 'position': 0,
#  'word_id': 12345, 'is_unknown': False, 'details': [...]}

json.dumps([token.to_dict() for token in tokens])
```

**戻り値:** `surface`、`byte_start`、`byte_end`、`position`、`word_id`、
`is_unknown`、`details` をキーに持つ `dict`。

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

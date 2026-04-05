# Tokenizer API

## TokenizerBuilder

`TokenizerBuilder` はビルダーパターンを使用して `Tokenizer` インスタンスを設定・構築します。

### コンストラクタ

#### `new TokenizerBuilder()`

デフォルト設定で新しいビルダーを作成します。

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
```

#### `new TokenizerBuilder().fromFile(filePath)`

JSON ファイルから設定を読み込み、新しいビルダーを返します。

```javascript
const builder = new TokenizerBuilder().fromFile("config.json");
```

### 設定メソッド

すべてのセッターメソッドはメソッドチェーンのために `this` を返します。

#### `setMode(mode)`

トークナイズモードを設定します。

- `"normal"` -- 標準的なトークナイズ（デフォルト）
- `"decompose"` -- 複合語をより小さな単位に分解

```javascript
builder.setMode("normal");
```

#### `setDictionary(path)`

システム辞書のパスまたは URI を設定します。

```javascript
// 埋め込み辞書を使用
builder.setDictionary("embedded://ipadic");

// 外部辞書を使用
builder.setDictionary("/path/to/dictionary");
```

#### `setUserDictionary(uri)`

ユーザー辞書の URI を設定します。

```javascript
builder.setUserDictionary("/path/to/user_dictionary");
```

#### `setKeepWhitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。

```javascript
builder.setKeepWhitespace(true);
```

#### `appendCharacterFilter(kind, args?)`

前処理パイプラインに文字フィルタを追加します。

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(kind, args?)`

後処理パイプラインにトークンフィルタを追加します。

```javascript
builder.appendTokenFilter("lowercase", {});
```

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```javascript
const tokenizer = builder.build();
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `new Tokenizer(dictionary, mode?, userDictionary?)`

読み込み済みの辞書から直接トークナイザーを作成します。

```javascript
const { Tokenizer, loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");
```

### Tokenizer メソッド

#### `tokenize(text)`

入力テキストをトークナイズし、`Token` オブジェクトの配列を返します。

```javascript
const tokens = tokenizer.tokenize("形態素解析");
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `string` | トークナイズするテキスト |

**戻り値:** `Token[]`

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

N-best トークナイズ結果を返します。各結果はトークン配列とトータルパスコストを含みます。

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const { tokens, cost } of results) {
  console.log(cost, tokens.map((t) => t.surface));
}
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `string` | トークナイズするテキスト |
| `n` | `number` | 返す結果の数 |
| `unique` | `boolean` | 結果の重複を排除（デフォルト: `false`） |
| `costThreshold` | `number \| undefined` | 最良パスからの最大コスト差（デフォルト: `undefined`） |

**戻り値:** `Array<{ tokens: Token[], cost: number }>`

## Token

`Token` は単一の形態素トークンを表します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `string` | トークンの表層形 |
| `byteStart` | `number` | 元テキストでの開始バイト位置 |
| `byteEnd` | `number` | 元テキストでの終了バイト位置 |
| `position` | `number` | トークンの位置インデックス |
| `wordId` | `number` | 辞書の単語 ID |
| `isUnknown` | `boolean` | 辞書に登録されていない単語の場合 `true` |
| `details` | `string[] \| null` | 形態素の詳細情報（品詞、読みなど） |

### Token メソッド

#### `getDetail(index)`

指定されたインデックスの詳細文字列を返します。インデックスが範囲外の場合は `null` を返します。

```javascript
const token = tokenizer.tokenize("東京")[0];
const pos = token.getDetail(0);      // 例: "名詞"
const subpos = token.getDetail(1);   // 例: "固有名詞"
const reading = token.getDetail(7);  // 例: "トウキョウ"
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `index` | `number` | details 配列へのゼロベースインデックス |

**戻り値:** `string | null`

`details` の構造は辞書によって異なります：

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: UniDic 仕様に準拠した詳細な形態素情報
- **ko-dic / CC-CEDICT / Jieba**: 各辞書固有の詳細フォーマット

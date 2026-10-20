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

YAML ファイルから設定を読み込み、新しいビルダーを返します。

```javascript
const builder = new TokenizerBuilder().fromFile("lindera.yml");
```

辞書、文字フィルタ、トークンフィルタの設定を含む完全な設定ファイルの例は
[`lindera-nodejs/resources/lindera.yml`](https://github.com/lindera/lindera/blob/main/lindera-nodejs/resources/lindera.yml)
を参照してください。

### 設定メソッド

すべてのセッターメソッドはビルダー自身（`this`）を返すため、メソッドチェーンでも 1 文ずつの呼び出しでも設定できます。

#### `setMode(mode)`

トークナイズモードを [`Mode`](#mode) で設定します。

- `Mode.normal()` -- 標準的なトークナイズ（デフォルト）
- `Mode.decompose(penalty?)` -- 複合語をより小さな単位に分解。モードが持つ [`Penalty`](#penalty) を使う

モード名 `"normal"` など、`Mode` でない値を渡すと `InvalidArg` のエラーを投げます。設定から読み込んだ値のように実行時に得たモード名は、[`Mode.fromStr()`](#modefromstrname) で `Mode` に変換します。

```javascript
const { Mode } = require("lindera");

builder.setMode(Mode.normal());
builder.setMode(Mode.decompose({ kanjiPenaltyLengthPenalty: 0 }));
builder.setMode(Mode.fromStr("decompose"));
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
builder.setUserDictionary("/path/to/user_dictionary.csv");
```

#### `setKeepWhitespace(keep)`

出力に空白トークンを含めるかどうかを制御します。残した空白はラティスにも残るため、他のトークンも、MeCab と同様に空白をラティス上で読み飛ばすデフォルトの出力と変わることがあります。空白を出力せずにラティスにだけ残すには、代わりに [`setSkipWhitespace(false)`](#setskipwhitespaceskip) を使います。

```javascript
builder.setKeepWhitespace(true);
```

#### `setSkipWhitespace(skip)`

空白をラティス上で読み飛ばすかどうかを設定します。値の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.skip_whitespace` と同じです。

| 値 | 効果 |
| --- | --- |
| `true` | 空白を読み飛ばし、MeCab と同様に空白の前後の語を直接接続する |
| `false` | 空白を独立したノード（空白の見出し語か `SPACE` の未知語）としてラティスに残す（v7 より前の動作）。空白は引き続き出力から除外する |
| `null` または `undefined` | デフォルト。辞書の `metadata.json` の設定に従う（SudachiDict 以外では読み飛ばす） |

ここで設定した値は、辞書のデフォルトと、`fromFile` で読み込んだ `skip_whitespace` より優先されます。空白トークンを残すとき（`setKeepWhitespace(true)`）は効果がありません。残した空白は常にラティスに残るためです。引数は省略できず、それ以外の値を渡すと `Error` を投げます。

たとえば IPADIC では、`Google が 新しい` の `が` は、デフォルトでは助詞（`助詞`）、`setSkipWhitespace(false)` では接続詞（`接続詞`）と解析されます。詳しくは [Segmenter](../lindera/segmenter.md#空白文字の扱い) を参照してください。

```javascript
// v7 より前と同じく、空白をラティスに残す
builder.setSkipWhitespace(false);

// 辞書のデフォルトに戻す
builder.setSkipWhitespace(null);
```

#### `setSpacePenalty(value)`

韓国語向けの左側空白ペナルティ（mecab-ko の `left-space-penalty-factor`）を設定します。直前に空白があり、先頭品詞タグがルールの一覧にある候補に、そのルールのコストを加算します。値の意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.space_penalty` と同じです。

| 値 | 効果 |
| --- | --- |
| `null` または `undefined` | デフォルト。辞書が `metadata.json` に同梱するルールがあればそれを使用 |
| `false` | ペナルティをオフにする |
| `true` | 辞書のルールを要求する。辞書がルールを同梱していない場合は `build()` が例外を投げる |
| `{ rules: [{ pos: [...], cost: n }, ...] }` | 辞書のルールの代わりにこのルールを適用する |

それ以外の値を渡すと `Error` を投げます。

```javascript
// ペナルティをオフにする
builder.setSpacePenalty(false);

// 明示的なルール（ko-dic が同梱するルールと同じ）
builder.setSpacePenalty({
  rules: [
    { pos: ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], cost: 3000 },
    { pos: ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], cost: 6000 },
  ],
});

// 辞書のデフォルトに戻す
builder.setSpacePenalty(null);
```

> [!NOTE]
> v6.1.0 以降、ko-dic では左側空白ペナルティがデフォルトで適用されます。ko-dic の `metadata.json` が mecab-ko-dic のルールを同梱しているためです。これにより韓国語の出力が変わります（例: `검색 이 잘 된다` の `이` は、ペナルティがあると感動詞 `IC`、なければ主格助詞 `JKS` になる）。オフにするには `setSpacePenalty(false)` を呼び出してください。直接作成する `Tokenizer` では、4 番目の引数に `false` を渡します（`new Tokenizer(dictionary, Mode.normal(), null, false)`）。v6.0.0 リリースからダウンロードした ko-dic はルールを同梱していないため、その辞書ではペナルティはオフのままで、`setSpacePenalty(true)` を指定すると `build()` が例外を投げます。詳細は [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

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

`setSpacePenalty`、`appendCharacterFilter`、`appendTokenFilter` の引数は、配列とオブジェクトの入れ子が 128 段までです。それより深い値や、自分自身を含む値を渡すと `Error` を投げます。

### ビルド

#### `build()`

設定された内容で `Tokenizer` をビルドして返します。

```javascript
const tokenizer = builder.build();
```

## Tokenizer

`Tokenizer` はテキストに対して形態素解析を行います。

### Tokenizer の作成

#### `new Tokenizer(dictionary, mode?, userDictionary?, spacePenalty?)`

読み込み済みの辞書から直接トークナイザーを作成します。

`mode` には、[`setMode`](#setmodemode) と同じく [`Mode`](#mode) を渡します。エラーも `setMode` と同じです。`null` または `undefined`（デフォルト）は normal モードです。ペナルティを指定した decompose モードには `Mode.decompose(penalty)` を渡します。

`spacePenalty` には [`setSpacePenalty`](#setspacepenaltyvalue) と同じ値を渡せます。`null` または `undefined`（デフォルト）では、辞書が同梱する左側空白ペナルティのルールを使います。不正な設定では、`setSpacePenalty` や `build()` と同じくエラーを投げます。

```javascript
const { Mode, Tokenizer, loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, Mode.normal());

// 左側空白ペナルティなしの ko-dic
const koTokenizer = new Tokenizer(loadDictionary("embedded://ko-dic"), Mode.normal(), null, false);
```

### Tokenizer メソッド

#### `tokenize(text)`

入力テキストをトークナイズし、プレーンなトークンオブジェクトの配列を返します。

```javascript
const tokens = tokenizer.tokenize("形態素解析");
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `string` | トークナイズするテキスト |

**戻り値:** `Token[]`

トークンはクラスのインスタンスではなくプレーンな JavaScript オブジェクトです。そのため通常の GC で回収され、`JSON.stringify`、`structuredClone`、worker への転送を変換なしで通過します。詳細は [Token](#token) を参照してください。

#### `tokenizeSurfaces(text)`

入力テキストをトークナイズし、トークンの surface のみを文字列の配列として返します。分かち書き用途の高速パスです。トークンオブジェクトを構築せず、形態素の詳細情報もロードしないため、surface 文字列だけが必要な場合は `tokenize` より大幅に高速です。結果は `tokenizer.tokenize(text).map((t) => t.surface)` と一致します。

```javascript
const surfaces = tokenizer.tokenizeSurfaces("形態素解析");
// ["形態素", "解析"]
```

**パラメータ:**

| 名前 | 型 | 説明 |
| --- | --- | --- |
| `text` | `string` | トークナイズするテキスト |

**戻り値:** `string[]`

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
| `n` | `number` | 返す結果の数。0 以上の整数 |
| `unique` | `boolean` | 結果の重複を排除（デフォルト: `false`） |
| `costThreshold` | `number \| undefined` | 最良パスからの最大コスト差（デフォルト: `undefined`） |

**戻り値:** `NbestResult[]`。各 `NbestResult` は `{ tokens: Token[], cost: number }` です。

## Token

`Token` は単一の形態素トークンを表すプレーンオブジェクトです。クラスではなく
TypeScript の `interface` であり、メソッドもプロトタイプも持ちません。
各フィールドは直接読み出します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `string` | トークンの表層形 |
| `byteStart` | `number` | 元テキストでの開始バイト位置 |
| `byteEnd` | `number` | 元テキストでの終了バイト位置 |
| `position` | `number` | トークンの位置インデックス |
| `wordId` | `number` | 辞書の単語 ID |
| `isUnknown` | `boolean` | 辞書に登録されていない単語の場合 `true` |
| `details` | `string[]` | 形態素の詳細情報（品詞、読みなど） |

### 詳細情報の読み出し

`details` を直接インデックスします。範囲外のインデックスは `undefined` になります。

```javascript
const token = tokenizer.tokenize("東京")[0];
const pos = token.details[0];      // 例: "名詞"
const subpos = token.details[1];   // 例: "固有名詞"
const reading = token.details[7];  // 例: "トウキョウ"
```

`details` の構造は辞書によって異なります：

- **IPADIC**: `[品詞, 品詞細分類1, 品詞細分類2, 品詞細分類3, 活用型, 活用形, 原形, 読み, 発音]`
- **UniDic**: UniDic 仕様に準拠した詳細な形態素情報
- **ko-dic / CC-CEDICT / Jieba**: 各辞書固有の詳細フォーマット

## Mode と Penalty

`TokenizerBuilder.setMode()` と `Tokenizer` のコンストラクタは、モードを [`Mode`](#mode) で受け取ります。decompose モードのペナルティは、`Mode` が [`Penalty`](#penalty) として保持します。

### `Mode`

トークナイズモードを表すクラスです。normal モードか、[`Penalty`](#penalty) を持つ decompose モードのどちらかです。Rust の `lindera::mode::Mode` に対応し（[Segmenter](../lindera/segmenter.md#トークナイズモード) を参照）、[`setMode()`](#setmodemode) と [`Tokenizer` のコンストラクタ](#new-tokenizerdictionary-mode-userdictionary-spacepenalty) が受け取ります：

- normal モード -- 辞書コストに基づく標準的なトークナイズ
- decompose モード -- ペナルティベースの複合語分解

`Mode` にはコンストラクタがなく、`new Mode()` はエラーを投げます。以下の静的ファクトリメソッドで作成します。作成したモードは変更できません。

#### `Mode.normal()`

normal モードを作成します。

```javascript
const { Mode } = require("lindera");

const mode = Mode.normal();
```

#### `Mode.decompose(penalty?)`

`penalty` を持つ decompose モードを作成します。`penalty` は [`Penalty`](#penalty) オブジェクトです。`null` または `undefined`（デフォルト）では既定のペナルティを使い、省略したフィールドはデフォルト値になります。オブジェクトでない値や、範囲内の整数でないフィールドを渡すと、`InvalidArg` のエラーを投げます。

```javascript
const decompose = Mode.decompose(); // 既定のペナルティ
const custom = Mode.decompose({ kanjiPenaltyLengthPenalty: 0 });
```

#### `Mode.fromStr(name)`

Rust の `Mode` と同じ規則で、名前からモードを作成します。`"normal"` は normal モード、`"decompose"` は既定のペナルティを持つ decompose モードです。名前の大文字と小文字は区別し、`"Normal"` や `"Decompose"` を含むそれ以外の名前では、メッセージに `Invalid mode` を含む `Error` を投げます。設定から読み込んだ値のように、実行時に得たモード名を変換するときに使います：

```javascript
const mode = Mode.fromStr("decompose");
console.log(mode.equals(Mode.decompose()));
// true
```

#### プロパティとメソッド

| メンバー | 型 | 説明 |
| --- | --- | --- |
| `name` | `string` | モードの名前。`"normal"` または `"decompose"`（読み取り専用のプロパティ） |
| `penalty` | `Penalty \| null` | decompose モードのペナルティ。すべてのフィールドを設定したコピーで、normal モードでは `null`（読み取り専用のプロパティ）。コピーを変更してもモードは変わらない |
| `isNormal()` | `boolean` | normal モードかどうか |
| `isDecompose()` | `boolean` | decompose モードかどうか |
| `equals(other)` | `boolean` | `other` と名前とペナルティが等しいかどうか。`Mode` でない値を渡すと `InvalidArg` のエラーを投げる |
| `toString()` | `string` | モードの名前（`name` と同じ） |

`===` はモードではなくオブジェクトを比べます。`Mode.normal()` は呼び出すたびに新しいオブジェクトを返すため、モードは `equals` で比べてください。

```javascript
const { Mode } = require("lindera");

const mode = Mode.decompose({ kanjiPenaltyLengthPenalty: 0 });
console.log(mode.name, mode.isNormal(), mode.isDecompose());
// decompose false true

// すべてのフィールドを設定したコピー。省略したフィールドはデフォルト値になる
const penalty = mode.penalty;
console.log(penalty.kanjiPenaltyLengthPenalty, penalty.otherPenaltyLengthPenalty);
// 0 1700

// コピーを変更してもモードは変わらない
penalty.otherPenaltyLengthPenalty = 0;
console.log(mode.penalty.otherPenaltyLengthPenalty);
// 1700

console.log(Mode.normal().penalty);
// null

console.log(Mode.decompose().equals(Mode.fromStr("decompose")));
// true
console.log(mode.equals(Mode.decompose()));
// false
console.log(`${Mode.normal()}`);
// normal
```

### `Penalty`

decompose モードのペナルティパラメータを持つプレーンなオブジェクト（TypeScript の型は `Penalty`）です。フィールドはすべて省略でき、省略したフィールドはデフォルト値になります：

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `kanjiPenaltyLengthThreshold` | `number` | ペナルティを適用する前の漢字列の長さの閾値。0 以上の整数（デフォルト: `2`） |
| `kanjiPenaltyLengthPenalty` | `number` | 長い漢字列に対するペナルティ値。32 ビット符号付き整数（デフォルト: `3000`） |
| `otherPenaltyLengthThreshold` | `number` | ペナルティを適用する前のその他の文字列の長さの閾値。0 以上の整数（デフォルト: `7`） |
| `otherPenaltyLengthPenalty` | `number` | 長いその他の文字列に対するペナルティ値。32 ビット符号付き整数（デフォルト: `1700`） |

`Penalty` は [`Mode.decompose()`](#modedecomposepenalty) に渡し、作成したモードを [`setMode()`](#setmodemode) か [`Tokenizer` のコンストラクタ](#new-tokenizerdictionary-mode-userdictionary-spacepenalty) に渡します。長さのペナルティをなくすと、decompose モードでも normal モードと同じく長い語が分割されずに残ります：

```javascript
const { Mode, Tokenizer, TokenizerBuilder, loadDictionary } = require("lindera");

const text = "関西国際空港限定トートバッグ";
const penalty = { kanjiPenaltyLengthPenalty: 0, otherPenaltyLengthPenalty: 0 };
const surfaces = (tokenizer) => tokenizer.tokenize(text).map((token) => token.surface);

const builder = new TokenizerBuilder().setDictionary("embedded://ipadic");
console.log(surfaces(builder.setMode(Mode.normal()).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose()).build()));
// [ '関西', '国際', '空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose(penalty)).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"), Mode.decompose(penalty));
console.log(surfaces(tokenizer));
// [ '関西国際空港', '限定', 'トートバッグ' ]
```

# Tokenizer API

このページでは、lindera-wasm が公開する JavaScript/TypeScript API について説明します。

## TokenizerBuilder

設定済みの `Tokenizer` インスタンスを作成するためのビルダークラスです。

### コンストラクタ

```javascript
const builder = new TokenizerBuilder();
```

デフォルト設定で新しいビルダーを作成します。

### メソッド

すべてのセッターは同じ設定を共有するビルダーハンドルを返すため、
`builder.setMode(Mode.normal()).setDictionary(...)` のようにチェーンでも、1 文ずつでも記述できます。
返されるハンドルは新しいオブジェクトですが、設定先は同一のビルダーです。

#### `setMode(mode)`

トークナイズモードを設定します。モードを設定しないビルダーは normal モードを使います。

- **パラメータ**: `mode` (Mode) -- [`Mode`](#mode)。`Mode.normal()` か、decompose モードの [`Penalty`](#penalty) を指定した `Mode.decompose()` です。渡した `Mode` はそのまま使えるので、同じものを何度でも渡せます。
- **戻り値**: `TokenizerBuilder`

モード名など `Mode` でない値を渡すと、`Error("expected instance of Mode")` が投げられます。名前でモードを選ぶには、[`Mode.fromStr()`](#mode) でモードを作成してください。

```javascript
builder.setMode(Mode.normal());
builder.setMode(Mode.decompose(new Penalty(undefined, 0)));
builder.setMode(Mode.fromStr("decompose"));
```

#### `setDictionary(uri)`

トークナイズに使用する辞書を設定します。`setDictionaryInstance()` で設定した辞書は、この辞書に置き換わります。

- **パラメータ**: `uri` (string) -- 辞書の URI（例: `"embedded://ipadic"`）
- **戻り値**: `TokenizerBuilder`

```javascript
builder.setDictionary("embedded://ipadic");
```

#### `setDictionaryInstance(dictionary)`

読み込み済みの辞書インスタンスをトークナイズに設定します。
URI の代わりにバイトデータから読み込んだ辞書（例: `loadDictionaryFromBytes()` 経由）を使用する場合に使います。
`setDictionary()` で設定した辞書は、この辞書に置き換わります（2 つのうち最後に呼んだ方の辞書が使われます）。ビルダーのほかの設定（モード、`setUserDictionaryInstance()`、`setKeepWhitespace()`、`setSkipWhitespace()`、`setSpacePenalty()`、追加したフィルタ）は、URI で設定した辞書と同じようにこの辞書にも適用されます。

- **パラメータ**: `dictionary` (Dictionary) -- 読み込み済みの辞書オブジェクト
- **戻り値**: `TokenizerBuilder`

```javascript
import { loadDictionaryFromBytes } from 'lindera-wasm';
import { loadDictionaryFiles } from 'lindera-wasm/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictTrie, files.dictValsIdx, files.dictVals,
    files.dictWordsIdx, files.dictWords, files.matrixMtx, files.charDef,
    files.unk,
);

builder.setDictionaryInstance(dictionary);
```

#### `setUserDictionaryInstance(userDictionary)`

読み込み済みのユーザー辞書インスタンスを設定します。`loadUserDictionaryFromBytesFor()` か `loadUserDictionaryFromBytes()`（CSV）、または `loadUserDictionaryBinFromBytes()`（ビルド済み `.bin`）でバイト列から読み込んでください。WebAssembly では URI ベースのユーザー辞書は使用できません。ユーザー辞書は、`setDictionary()` と `setDictionaryInstance()` のどちらで設定した辞書にも適用されます。

- **パラメータ**: `userDictionary` (UserDictionary) -- 読み込み済みのユーザー辞書オブジェクト
- **戻り値**: `TokenizerBuilder`

#### `setKeepWhitespace(keep)`

出力に空白トークンを保持するかどうかを設定します。保持した空白はラティスにも残るため、他のトークンも、MeCab と同様に空白をラティス上で読み飛ばすデフォルトの出力と変わることがあります。空白を出力せずにラティスにだけ残すには、代わりに [`setSkipWhitespace(false)`](#setskipwhitespaceskip) を使います。

- **パラメータ**: `keep` (boolean) -- `true` で空白トークンを保持
- **戻り値**: `TokenizerBuilder`

```javascript
builder.setKeepWhitespace(true);
```

#### `setSkipWhitespace(skip)`

空白をラティス上で読み飛ばすかどうかを設定します。意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.skip_whitespace` と同じです。`true` では空白を読み飛ばし、MeCab と同様に空白の前後の語を直接接続します。`false` では空白を独立したノード（空白の見出し語か `SPACE` の未知語）としてラティスに残します（v7 より前の動作。空白は引き続き出力から除外されます）。`null` または `undefined` では、辞書の `metadata.json` のデフォルトに戻ります（SudachiDict 以外では読み飛ばします）。

ここで設定した値は、`setDictionary()` と `setDictionaryInstance()` のどちらで設定した辞書でも、辞書のデフォルトより優先されます。空白トークンを保持するとき（`setKeepWhitespace(true)`）は効果がありません。保持した空白は常にラティスに残るためです。たとえば IPADIC では、`Google が 新しい` の `が` は、デフォルトでは助詞（`助詞`）、`setSkipWhitespace(false)` では接続詞（`接続詞`）と解析されます。詳しくは [Segmenter](../lindera/segmenter.md#空白文字の扱い) を参照してください。

- **パラメータ**: `skip` (boolean | null | undefined) -- 設定値。`null` または `undefined` で辞書のデフォルト。TypeScript の宣言では引数を省略できません。それ以外の値は JavaScript の真偽値の規則で扱われます
- **戻り値**: `TokenizerBuilder`

```javascript
// v7 より前と同じく、空白をラティスに残す
builder.setSkipWhitespace(false);
```

#### `setSpacePenalty(value)`

韓国語向けの左側空白ペナルティ（left-space penalty）を設定します。直前に空白がある候補のうち、本来は前の語に空白なしで付く品詞（助詞、語尾など）の候補にコストを加算します。

- **パラメータ**: `value` (boolean | object | null | undefined) -- 設定値。意味は[設定ファイル](../lindera-analysis/configuration.md)の `segmenter.space_penalty` と同じです
- **戻り値**: `TokenizerBuilder`

`value` は次のいずれかの形式をとります。それ以外の値ではエラー文字列が投げられます：

- `null` または `undefined` -- デフォルト。辞書が `metadata.json` に同梱するルールがあればそれを使います（ko-dic は mecab-ko-dic のルールを同梱し、他の同梱辞書はルールを持ちません）。以前の呼び出しを取り消すときにも使います。
- `false` -- ペナルティをオフにします。
- `true` -- 辞書のルールを要求します。ルールを同梱しない辞書では `build()` がエラーを投げます。
- `{ rules: [{ pos: [...], cost: n }, ...] }` 形式のオブジェクト -- 辞書のルールの代わりにこのルールを適用します。先頭品詞タグが `pos` に含まれる候補に `cost` を加算し、最初に一致したルールが使われます。

この設定は、`setDictionary()` で設定した辞書にも、`setDictionaryInstance()` で設定した辞書（たとえば `loadDictionaryFromBytes()` で OPFS から読み込んだ ko-dic）にも適用されます。辞書オブジェクト自体は変更されません。`Tokenizer` コンストラクタは、同じ設定を引数 `spacePenalty` で受け取ります。

```javascript
// ペナルティをオフにする
builder.setSpacePenalty(false);

// 明示的なルール
builder.setSpacePenalty({
    rules: [
        { pos: ["EC", "EF", "EP", "ETM", "ETN", "VCP", "XSA", "XSN", "XSV"], cost: 3000 },
        { pos: ["JC", "JKB", "JKC", "JKG", "JKO", "JKQ", "JKS", "JKV", "JX"], cost: 6000 },
    ],
});
```

> [!NOTE]
> v6.1.0 から、ko-dic では mecab-ko と同じく左側空白ペナルティがデフォルトで適用されます。たとえば `검색 이 잘 된다` の `이` は感動詞 `IC` と解析されます（ペナルティがなければ主格助詞 `JKS`）。オフにするには `builder.setSpacePenalty(false)` を呼び出してください。ルールを同梱しているのは Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースの ko-dic ではペナルティはオフのままで、辞書を再ビルドするまで `true` はエラーになります。詳しくは [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

#### `appendCharacterFilter(name, args)`

前処理パイプラインに文字フィルタを追加します。

- **パラメータ**:
  - `name` (string) -- フィルタ名（例: `"unicode_normalize"`、`"japanese_iteration_mark"`）
  - `args` (object, 省略可) -- フィルタの設定
- **戻り値**: `TokenizerBuilder`

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(name, args)`

後処理パイプラインにトークンフィルタを追加します。

- **パラメータ**:
  - `name` (string) -- フィルタ名（例: `"japanese_stop_tags"`、`"lowercase"`）
  - `args` (object, 省略可) -- フィルタの設定
- **戻り値**: `TokenizerBuilder`

```javascript
builder.appendTokenFilter("japanese_stop_tags", {
    tags: ["助詞,格助詞,一般", "助詞,係助詞", "助詞,連体化", "助動詞", "記号,句点", "記号,読点"]
});
```

`setSpacePenalty`、`appendCharacterFilter`、`appendTokenFilter` の引数は、配列、オブジェクト、`Map` の入れ子が 128 段までです。それより深い値や、自分自身を含む値を渡すと、エラー文字列が投げられます。

#### `build()`

設定済みの `Tokenizer` インスタンスをビルドして返します。辞書を `setDictionary()` と `setDictionaryInstance()` のどちらで設定しても、すべての設定が適用されます。ビルド後もビルダーはそのまま使えるため、同じ設定から複数のトークナイザーをビルドできます。

- **戻り値**: `Tokenizer`

```javascript
const tokenizer = builder.build();
```

## Tokenizer

メインのトークナイザークラスです。`TokenizerBuilder.build()` またはコンストラクタ経由で作成できます。

### Tokenizer コンストラクタ

```javascript
const tokenizer = new Tokenizer(dictionary, mode, userDictionary, spacePenalty);
```

- **パラメータ**:
  - `dictionary` (Dictionary) -- 読み込み済みの辞書オブジェクト
  - `mode` (Mode, 省略可) -- [`setMode()`](#setmodemode) と同じ [`Mode`](#mode)。`null` または `undefined`（デフォルト）では normal モードになります。渡した `Mode` はそのまま使えるので、同じものを複数のトークナイザーやビルダーに渡せます。`Mode` でない値を渡すと、`Error("expected instance of Mode")` が投げられます。
  - `userDictionary` (UserDictionary, 省略可) -- 読み込み済みのユーザー辞書
  - `spacePenalty` (boolean | object, 省略可) -- 左側空白ペナルティ。[`setSpacePenalty()`](#setspacepenaltyvalue) と同じ値を渡せます。`null` または `undefined`（デフォルト）では、辞書が同梱するルールを使います。不正な設定では、`setSpacePenalty()` や `build()` と同じくエラー文字列が投げられます。

### Tokenizer メソッド

#### `tokenize(text)`

入力テキストをトークナイズします。

- **パラメータ**: `text` (string) -- トークナイズするテキスト
- **戻り値**: `Token[]` -- トークンオブジェクトの配列

```javascript
const tokens = tokenizer.tokenize("関西国際空港");
```

#### `tokenizeSurfaces(text)`

入力テキストをトークナイズし、トークンの surface のみを返します。分かち書き用途の高速パスです。トークンオブジェクトを生成せず、形態素の詳細情報もロードしないため、surface 文字列だけが必要な場合は `tokenize` より大幅に高速です。結果は `tokenizer.tokenize(text).map((t) => t.surface)` と一致します。（Web Worker とは無関係です。）

- **パラメータ**: `text` (string) -- トークナイズするテキスト
- **戻り値**: `string[]` -- surface 文字列の配列

```javascript
const surfaces = tokenizer.tokenizeSurfaces("関西国際空港");
// ["関西国際空港"]
```

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

トータルパスコスト順に N-best トークナイズ結果を返します。

- **パラメータ**:
  - `text` (string) -- トークナイズするテキスト
  - `n` (number) -- 返す結果の数。0 以上の整数
  - `unique` (boolean, 省略可) -- 同一のセグメンテーション結果を重複排除（デフォルト: `false`）
  - `costThreshold` (bigint, 省略可) -- `bestCost + threshold` 以内のパスのみ返す。64 ビットに収まる値
- **戻り値**: `{ tokens: Token[], cost: number }` の配列

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);

// コスト閾値を指定する場合 -- bigint リテラルとして渡す必要がある点に注意
const resultsWithThreshold = tokenizer.tokenizeNbest("すもももももももものうち", 3, false, 100n);
```

## Token

トークナイザーが生成する単一のトークンで、プレーンな JavaScript オブジェクトです。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `string` | トークンの表層形 |
| `byteStart` | `number` | 元テキストでの開始バイトオフセット |
| `byteEnd` | `number` | 元テキストでの終了バイトオフセット |
| `position` | `number` | トークンの位置インデックス |
| `wordId` | `number` | 辞書内の単語 ID |
| `isUnknown` | `boolean` | 未知語かどうか |
| `details` | `string[]` | 形態素の詳細フィールド |

> [!NOTE]
> トークンは `wasm_bindgen` のクラスインスタンスではなくプレーンオブジェクトです。クラスインスタンスは JavaScript 側が解放するまでデータを Rust ヒープ上に保持するため、yield しない同期ループでトークナイズするとメモリが蓄積します。プレーンオブジェクトなら Rust 側に確保されるものが無く、結果は `JSON.stringify`、`structuredClone`、worker への転送を変換なしで通過します。フィールド名は `lindera-nodejs` バインディングと同じ camelCase です。

### 詳細情報の読み出し

`details` を直接インデックスします。範囲外のインデックスは `undefined` になります。

```javascript
const pos = token.details[0];     // 例: "名詞"
const reading = token.details[7]; // 例: "トウキョウ"
```

### トークンのシリアライズ

トークンは既にプレーンオブジェクトなので、そのままシリアライズできます。

```javascript
console.log(JSON.stringify(token, null, 2));
```

## ヘルパー関数

> [!NOTE]
> 以下の例は `lindera-wasm-ipadic` からインポートしていますが、これは `embed-ipadic` feature を使ってローカルビルドした場合の説明用パッケージ名であり、npm に公開されているものではありません。実際に公開されているのは `lindera-wasm` のみです。詳細は [npm パッケージの命名規則](./installation.md#npm-パッケージの命名規則) を参照してください。

### `loadDictionary(uri)`

指定された URI から辞書を読み込みます。

- **パラメータ**: `uri` (string) -- 辞書の URI（例: `"embedded://ipadic"`）
- **戻り値**: `Dictionary`

```javascript
import { loadDictionary } from 'lindera-wasm-ipadic';

const dict = loadDictionary("embedded://ipadic");
```

### `loadUserDictionaryFromBytes(csv, metadata)`

CSV バイト列（UTF-8）からユーザー辞書をビルドします。バイト列は `fetch`・ファイル入力・OPFS から取得します。

- **パラメータ**:
  - `csv` (Uint8Array) -- ユーザー辞書 CSV の内容
  - `metadata` (Metadata) -- 組み合わせるシステム辞書のメタデータ（例: `dictionary.metadata`）
- **戻り値**: `UserDictionary`

### `loadUserDictionaryFromBytesFor(csv, dictionary)`

読み込み済みのシステム辞書に合わせて、CSV バイト列（UTF-8）からユーザー辞書をビルドします。CSV はその辞書のメタデータでビルドし、接続コスト行列と照らして確認するので、文脈 ID が行列の範囲外にある行は、その CSV の行番号と表層形を示すエラーになります。辞書は借用するだけで、その後も使えます。

- **パラメータ**:
  - `csv` (Uint8Array) -- ユーザー辞書 CSV の内容
  - `dictionary` (Dictionary) -- 組み合わせるシステム辞書
- **戻り値**: `UserDictionary`

### `loadUserDictionaryBinFromBytes(bytes)`

ビルド済みユーザー辞書（`lindera build --user` の出力）をバイト列から読み込みます。

- **パラメータ**: `bytes` (Uint8Array) -- `.bin` の内容
- **戻り値**: `UserDictionary`

### `version()` / `getVersion()`

lindera-wasm パッケージのバージョン文字列を返します。

- **戻り値**: `string`

```javascript
import { version } from 'lindera-wasm-ipadic';

console.log(version()); // 例: "7.0.0"
```

## ユーティリティクラス

### Mode

トークナイズモードです。normal モードか、[`Penalty`](#penalty) を持つ decompose モードを表します。Rust の `Mode`（[トークナイズモード](../lindera/segmenter.md#トークナイズモード)を参照）に対応するクラスで、[`setMode()`](#setmodemode) と [`Tokenizer` のコンストラクタ](#tokenizer-コンストラクタ) が受け取ります。

`Mode` には公開のコンストラクタがありません。`new Mode()` は TypeScript ではエラーになり、JavaScript ではメソッドを呼ぶとエラーを投げるオブジェクトができます。モードは次の静的メソッドで作成します。

| 静的メソッド | 説明 |
| --- | --- |
| `Mode.normal()` | normal モード。辞書コストに基づく標準的なトークナイズ |
| `Mode.decompose(penalty?)` | decompose モード。ペナルティベースのセグメンテーションで複合語を分解します。`penalty` は [`Penalty`](#penalty) で、`null` または `undefined`（デフォルト）では既定のペナルティを使います。`Penalty` でない値を渡すと、エラー文字列（`penalty must be a Penalty`）が投げられます |
| `Mode.fromStr(name)` | `"normal"` または `"decompose"`（既定のペナルティ）という名前のモード。名前の大文字と小文字は区別され、それ以外の名前では `Invalid mode` を含むエラー文字列が投げられます |

| プロパティ / メソッド | 型 | 説明 |
| --- | --- | --- |
| `name` | `string` | `"normal"` または `"decompose"`（読み取り専用） |
| `penalty` | `Penalty` または `undefined` | decompose モードのペナルティのコピー。normal モードでは `undefined`（読み取り専用） |
| `isNormal()` | `boolean` | normal モードかどうか |
| `isDecompose()` | `boolean` | decompose モードかどうか |
| `equals(other)` | `boolean` | `other` と名前とペナルティが等しいかどうか。`Mode` でない値を渡すと `Error("expected instance of Mode")` が投げられます |
| `toString()` | `string` | `name` と同じ名前を返す |

`Mode` は変更できません。`Mode.decompose()` は `Penalty` のコピーを持ち、`penalty` はさらに別のコピーを返すので、あとからどちらの `Penalty` を変更してもモードは変わりません。`Mode.decompose()` に渡した `Penalty` も、`setMode()` や `Tokenizer` のコンストラクタに渡した `Mode` も、そのまま使えます。

```javascript
import { Mode, Penalty } from 'lindera-wasm-ipadic';

const mode = Mode.decompose(new Penalty(undefined, 0));
console.log(mode.name, mode.isNormal(), mode.isDecompose());
// decompose false true
console.log(`${mode}`, mode.penalty.kanji_penalty_length_penalty);
// decompose 0
console.log(mode.equals(Mode.decompose()), Mode.fromStr("decompose").equals(Mode.decompose()));
// false true
console.log(Mode.normal().penalty);
// undefined
```

### Penalty

decompose モードの設定です。複合語をどの程度積極的に分解するかを制御します。

```javascript
const penalty = new Penalty(
    kanjiThreshold?,     // 漢字の長さ閾値（デフォルト: 2）
    kanjiPenalty?,       // 漢字の長さペナルティ（デフォルト: 3000）
    otherThreshold?,     // その他の文字の長さ閾値（デフォルト: 7）
    otherPenalty?,       // その他の文字の長さペナルティ（デフォルト: 1700）
);
```

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `kanji_penalty_length_threshold` | `number` | `2` | 漢字複合語分割の長さ閾値 |
| `kanji_penalty_length_penalty` | `number` | `3000` | 閾値を超える漢字複合語のペナルティコスト |
| `other_penalty_length_threshold` | `number` | `7` | 非漢字複合語分割の長さ閾値 |
| `other_penalty_length_penalty` | `number` | `1700` | 閾値を超える非漢字複合語のペナルティコスト |

[`Mode.decompose()`](#mode) に渡し、そのモードを [`setMode()`](#setmodemode) か [`Tokenizer` のコンストラクタ](#tokenizer-コンストラクタ) に渡します。`Penalty` は読み取られるだけで消費されないので、同じものを何度でも渡せます。長さのペナルティをなくすと、decompose モードでも normal モードと同じく長い語が分割されずに残ります。

```javascript
import { Mode, Penalty, Tokenizer, TokenizerBuilder, loadDictionary } from 'lindera-wasm-ipadic';

const text = "関西国際空港限定トートバッグ";
const surfaces = (tokenizer) => tokenizer.tokenize(text).map((token) => token.surface);

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
console.log(surfaces(builder.setMode(Mode.normal()).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

console.log(surfaces(builder.setMode(Mode.decompose()).build()));
// [ '関西', '国際', '空港', '限定', 'トートバッグ' ]

const mode = Mode.decompose(new Penalty(undefined, 0, undefined, 0));
console.log(surfaces(builder.setMode(mode).build()));
// [ '関西国際空港', '限定', 'トートバッグ' ]

const tokenizer = new Tokenizer(loadDictionary("embedded://ipadic"), mode);
console.log(surfaces(tokenizer));
// [ '関西国際空港', '限定', 'トートバッグ' ]
```

### LinderaError

Lindera 操作のエラー型です。

```javascript
const error = new LinderaError("message");
console.log(error.message);    // "message"
console.log(error.toString()); // "message"
```

| プロパティ / メソッド | 型 | 説明 |
| --- | --- | --- |
| `message` | `string` | エラーメッセージ |
| `toString()` | `string` | エラーメッセージを返す |

> [!NOTE]
> `LinderaError` はユーティリティクラスとしてエクスポートされていますが、`TokenizerBuilder`・`Tokenizer`・`Mode`・辞書読み込み関数（`lindera-wasm/src/tokenizer.rs`、`lindera-wasm/src/mode.rs`、`lindera-wasm/src/dictionary.rs`）の実際のエラーパスはすべて `JsValue::from_str(...)` で reject しており、`JsLinderaError`/`LinderaError` のインスタンスではありません。そのため、これらの API が投げるエラーは JavaScript 側では単なる文字列として現れます。例外は、メソッドが受け取るクラスのインスタンスでない引数（`setMode()` に渡したモード名など）です。JS グルーが Rust を呼ぶ前にこれを確かめ、`expected instance of <クラス名>`（この例では `expected instance of Mode`）というメッセージの `Error` を投げます。`catch (e) { ... }` で捕捉する際は、`e` を `LinderaError` インスタンスではなく `string` か `Error` として扱ってください。

## snake\_case エイリアス

Python API との一貫性のため、次のメソッドは snake\_case 形式でも利用可能です：

| camelCase | snake\_case |
| --- | --- |
| `setMode()` | `set_mode()` |
| `setDictionary()` | `set_dictionary()` |
| `setDictionaryInstance()` | `set_dictionary_instance()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `setSkipWhitespace()` | `set_skip_whitespace()` |
| `setSpacePenalty()` | `set_space_penalty()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeSurfaces()` | `tokenize_surfaces()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytes()` | `load_user_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytesFor()` | `load_user_dictionary_from_bytes_for()` |
| `loadUserDictionaryBinFromBytes()` | `load_user_dictionary_bin_from_bytes()` |

`Mode` のメソッド（`fromStr()`、`isNormal()`、`isDecompose()`）には snake\_case のエイリアスはありません。

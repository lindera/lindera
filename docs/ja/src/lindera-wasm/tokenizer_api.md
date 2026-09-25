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
`builder.setMode("normal").setDictionary(...)` のようにチェーンでも、1 文ずつでも記述できます。
返されるハンドルは新しいオブジェクトですが、設定先は同一のビルダーです。

#### `setMode(mode)`

トークナイズモードを設定します。

- **パラメータ**: `mode` (string) -- `"normal"` または `"decompose"`
- **戻り値**: `TokenizerBuilder`

```javascript
builder.setMode("normal");
```

#### `setDictionary(uri)`

トークナイズに使用する辞書を設定します。

- **パラメータ**: `uri` (string) -- 辞書の URI（例: `"embedded://ipadic"`）
- **戻り値**: `TokenizerBuilder`

```javascript
builder.setDictionary("embedded://ipadic");
```

#### `setDictionaryInstance(dictionary)`

読み込み済みの辞書インスタンスをトークナイズに設定します。
URI の代わりにバイトデータから読み込んだ辞書（例: `loadDictionaryFromBytes()` 経由）を使用する場合に使います。

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

読み込み済みのユーザー辞書インスタンスを設定します。`loadUserDictionaryFromBytes()`（CSV）または `loadUserDictionaryBinFromBytes()`（ビルド済み `.bin`）でバイト列から読み込んでください。WebAssembly では URI ベースのユーザー辞書は使用できません。

- **パラメータ**: `userDictionary` (UserDictionary) -- 読み込み済みのユーザー辞書オブジェクト
- **戻り値**: `TokenizerBuilder`

#### `setKeepWhitespace(keep)`

出力に空白トークンを保持するかどうかを設定します。

- **パラメータ**: `keep` (boolean) -- `true` で空白トークンを保持
- **戻り値**: `TokenizerBuilder`

```javascript
builder.setKeepWhitespace(true);
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

この設定は、`setDictionary()` で設定した辞書にも、`setDictionaryInstance()` で設定した辞書（たとえば `loadDictionaryFromBytes()` で OPFS から読み込んだ ko-dic）にも適用されます。辞書オブジェクト自体は変更されません。`Tokenizer` コンストラクタで作成したトークナイザーは常に辞書のデフォルトを使います。設定を変えるには `TokenizerBuilder` でビルドしてください。

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
> v6.1.0 から、ko-dic では mecab-ko と同じく左側空白ペナルティがデフォルトで適用されます。たとえば `서울 시 에서` の `시` は、語尾 `EP` ではなく名詞 `NNG` と解析されるようになりました。v6.0 の出力に戻すには `builder.setSpacePenalty(false)` を呼び出してください。ルールを同梱しているのは Lindera 6.1.0 以降でビルドした ko-dic だけです。v6.0.0 リリースの ko-dic ではペナルティはオフのままで、辞書を再ビルドするまで `true` はエラーになります。詳しくは [Segmenter](../lindera/segmenter.md#左側空白ペナルティ韓国語) を参照してください。

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

#### `build()`

設定済みの `Tokenizer` インスタンスをビルドして返します。ビルド後もビルダーはそのまま使えるため、同じ設定から複数のトークナイザーをビルドできます。

- **戻り値**: `Tokenizer`

```javascript
const tokenizer = builder.build();
```

## Tokenizer

メインのトークナイザークラスです。`TokenizerBuilder.build()` またはコンストラクタ経由で作成できます。

### Tokenizer コンストラクタ

```javascript
const tokenizer = new Tokenizer(dictionary, mode, userDictionary);
```

- **パラメータ**:
  - `dictionary` (Dictionary) -- 読み込み済みの辞書オブジェクト
  - `mode` (string, 省略可) -- トークナイズモード（`"normal"` または `"decompose"`、デフォルト: `"normal"`）
  - `userDictionary` (UserDictionary, 省略可) -- 読み込み済みのユーザー辞書

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
  - `n` (number) -- 返す結果の数
  - `unique` (boolean, 省略可) -- 同一のセグメンテーション結果を重複排除（デフォルト: `false`）
  - `costThreshold` (bigint, 省略可) -- `bestCost + threshold` 以内のパスのみ返す
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

### `loadUserDictionaryBinFromBytes(bytes)`

ビルド済みユーザー辞書（`lindera build --user` の出力）をバイト列から読み込みます。

- **パラメータ**: `bytes` (Uint8Array) -- `.bin` の内容
- **戻り値**: `UserDictionary`

### `version()` / `getVersion()`

lindera-wasm パッケージのバージョン文字列を返します。

- **戻り値**: `string`

```javascript
import { version } from 'lindera-wasm-ipadic';

console.log(version()); // 例: "6.1.0"
```

## 列挙型とユーティリティクラス

### Mode

トークナイズモードの列挙型です。

| 値 | 説明 |
| --- | --- |
| `Mode.Normal` | 辞書コストに基づく標準的なトークナイズ |
| `Mode.Decompose` | ペナルティベースのセグメンテーションによる複合語分解 |

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
> `LinderaError` はユーティリティクラスとしてエクスポートされていますが、`TokenizerBuilder`・`Tokenizer`・辞書読み込み関数（`lindera-wasm/src/tokenizer.rs`、`lindera-wasm/src/dictionary.rs`）の実際のエラーパスはすべて `JsValue::from_str(...)` で reject しており、`JsLinderaError`/`LinderaError` のインスタンスではありません。そのため、これらの API が投げるエラーは JavaScript 側では単なる文字列として現れます。`catch (e) { ... }` で捕捉する際は、`e` を `LinderaError` インスタンスではなく `string` として扱ってください。

## snake\_case エイリアス

Python API との一貫性のため、すべてのメソッドは snake\_case 形式でも利用可能です：

| camelCase | snake\_case |
| --- | --- |
| `setMode()` | `set_mode()` |
| `setDictionary()` | `set_dictionary()` |
| `setDictionaryInstance()` | `set_dictionary_instance()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `setSpacePenalty()` | `set_space_penalty()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeSurfaces()` | `tokenize_surfaces()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionaryFromBytes()` | `load_user_dictionary_from_bytes()` |
| `loadUserDictionaryBinFromBytes()` | `load_user_dictionary_bin_from_bytes()` |

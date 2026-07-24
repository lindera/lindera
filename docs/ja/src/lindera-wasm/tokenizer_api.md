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

#### `setMode(mode)`

トークナイズモードを設定します。

- **パラメータ**: `mode` (string) -- `"normal"` または `"decompose"`
- **戻り値**: void

```javascript
builder.setMode("normal");
```

#### `setDictionary(uri)`

トークナイズに使用する辞書を設定します。

- **パラメータ**: `uri` (string) -- 辞書の URI（例: `"embedded://ipadic"`）
- **戻り値**: void

```javascript
builder.setDictionary("embedded://ipadic");
```

#### `setDictionaryInstance(dictionary)`

読み込み済みの辞書インスタンスをトークナイズに設定します。
URI の代わりにバイトデータから読み込んだ辞書（例: `loadDictionaryFromBytes()` 経由）を使用する場合に使います。

- **パラメータ**: `dictionary` (Dictionary) -- 読み込み済みの辞書オブジェクト
- **戻り値**: void

```javascript
import { loadDictionaryFromBytes } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

builder.setDictionaryInstance(dictionary);
```

#### `setUserDictionary(uri)`

ユーザー定義辞書を設定します。

- **パラメータ**: `uri` (string) -- ユーザー辞書のパスまたは URI
- **戻り値**: void

```javascript
builder.setUserDictionary("file:///path/to/user_dict.csv");
```

#### `setUserDictionaryInstance(userDictionary)`

読み込み済みのユーザー辞書インスタンスを設定します。URI の代わりにバイトデータから読み込んだユーザー辞書を使用する場合に使います。

- **パラメータ**: `userDictionary` (UserDictionary) -- 読み込み済みのユーザー辞書オブジェクト
- **戻り値**: void

#### `setKeepWhitespace(keep)`

出力に空白トークンを保持するかどうかを設定します。

- **パラメータ**: `keep` (boolean) -- `true` で空白トークンを保持
- **戻り値**: void

```javascript
builder.setKeepWhitespace(true);
```

#### `appendCharacterFilter(name, args)`

前処理パイプラインに文字フィルタを追加します。

- **パラメータ**:
  - `name` (string) -- フィルタ名（例: `"unicode_normalize"`、`"japanese_iteration_mark"`）
  - `args` (object, 省略可) -- フィルタの設定
- **戻り値**: void

```javascript
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
```

#### `appendTokenFilter(name, args)`

後処理パイプラインにトークンフィルタを追加します。

- **パラメータ**:
  - `name` (string) -- フィルタ名（例: `"japanese_stop_tags"`、`"lowercase"`）
  - `args` (object, 省略可) -- フィルタの設定
- **戻り値**: void

```javascript
builder.appendTokenFilter("japanese_stop_tags", {
    tags: ["助詞", "助動詞", "記号"]
});
```

#### `build()`

設定済みの `Tokenizer` インスタンスをビルドして返します。ビルダーは消費されます。

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

#### `tokenizeNbest(text, n, unique?, costThreshold?)`

トータルパスコスト順に N-best トークナイズ結果を返します。

- **パラメータ**:
  - `text` (string) -- トークナイズするテキスト
  - `n` (number) -- 返す結果の数
  - `unique` (boolean, 省略可) -- 同一のセグメンテーション結果を重複排除（デフォルト: `false`）
  - `costThreshold` (bigint, 省略可) -- `bestCost + threshold` 以内のパスのみ返す
- **戻り値**: `{ tokens: object[], cost: number }` の配列

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);

// コスト閾値を指定する場合 -- bigint リテラルとして渡す必要がある点に注意
const resultsWithThreshold = tokenizer.tokenizeNbest("すもももももももものうち", 3, false, 100n);
```

## Token

トークナイザーが生成する単一のトークンを表します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `surface` | `string` | トークンの表層形 |
| `byte_start` | `number` | 元テキストでの開始バイトオフセット |
| `byte_end` | `number` | 元テキストでの終了バイトオフセット |
| `position` | `number` | トークンの位置インデックス |
| `word_id` | `number` | 辞書内の単語 ID |
| `is_unknown` | `boolean` | 未知語かどうか |
| `details` | `string[]` | 形態素の詳細フィールド |

> [!NOTE]
> これらが `Token` オブジェクトの実際のフィールド名です -- `lindera-wasm/src/token.rs` は `js_name` によるリネームを一切行っていないため、JavaScript 側でも snake_case のままになります。camelCase にリネームされるのは、下記の `toJSON()` が返す JSON 用オブジェクトのみです。

### Token メソッド

#### `getDetail(index)`

指定されたインデックスの詳細文字列を返します。

- **パラメータ**: `index` (number) -- details 配列へのゼロベースインデックス
- **戻り値**: `string | undefined`

```javascript
const pos = token.getDetail(0);   // 例: "名詞"
const reading = token.getDetail(7); // 例: "トウキョウ"
```

#### `toJSON()`

トークンのプレーンな JavaScript オブジェクト表現を返します。

- **戻り値**: `surface`、`byteStart`、`byteEnd`、`position`、`wordId`、`isUnknown`、`details` をキーに持つ `object`

```javascript
console.log(JSON.stringify(token.toJSON(), null, 2));
```

## ヘルパー関数

> [!NOTE]
> 以下の例は `lindera-wasm-web-ipadic` からインポートしていますが、これは `embed-ipadic` feature を使ってローカルビルドした場合の説明用パッケージ名であり、npm に公開されているものではありません。実際に公開されているのは `lindera-wasm-web` と `lindera-wasm-bundler` のみです。詳細は [npm パッケージの命名規則](./installation.md#npm-パッケージの命名規則) を参照してください。

### `loadDictionary(uri)`

指定された URI から辞書を読み込みます。

- **パラメータ**: `uri` (string) -- 辞書の URI（例: `"embedded://ipadic"`）
- **戻り値**: `Dictionary`

```javascript
import { loadDictionary } from 'lindera-wasm-web-ipadic';

const dict = loadDictionary("embedded://ipadic");
```

### `loadUserDictionary(uri, metadata)`

指定された URI からユーザー辞書を読み込みます。

- **パラメータ**:
  - `uri` (string) -- ユーザー辞書ファイルのパスまたは URI
  - `metadata` (Metadata) -- 辞書のメタデータオブジェクト
- **戻り値**: `UserDictionary`

### `buildDictionary(inputDir, outputDir, metadata)`

ソースファイルからコンパイル済み辞書をビルドします。

- **パラメータ**:
  - `inputDir` (string) -- 辞書ソースファイルを含むディレクトリのパス
  - `outputDir` (string) -- 出力ディレクトリのパス
  - `metadata` (Metadata) -- 辞書のメタデータオブジェクト
- **戻り値**: void

### `buildUserDictionary(inputFile, outputDir, metadata?)`

CSV ファイルからコンパイル済みユーザー辞書をビルドします。

- **パラメータ**:
  - `inputFile` (string) -- ユーザー辞書 CSV ファイルのパス
  - `outputDir` (string) -- 出力ディレクトリのパス
  - `metadata` (Metadata, 省略可) -- 辞書のメタデータオブジェクト
- **戻り値**: void

### `version()` / `getVersion()`

lindera-wasm パッケージのバージョン文字列を返します。

- **戻り値**: `string`

```javascript
import { version } from 'lindera-wasm-web-ipadic';

console.log(version()); // 例: "4.0.1"
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
| `setUserDictionary()` | `set_user_dictionary()` |
| `setUserDictionaryInstance()` | `set_user_dictionary_instance()` |
| `setKeepWhitespace()` | `set_keep_whitespace()` |
| `appendCharacterFilter()` | `append_character_filter()` |
| `appendTokenFilter()` | `append_token_filter()` |
| `tokenizeNbest()` | `tokenize_nbest()` |
| `loadDictionary()` | `load_dictionary()` |
| `loadDictionaryFromBytes()` | `load_dictionary_from_bytes()` |
| `loadUserDictionary()` | `load_user_dictionary()` |
| `buildDictionary()` | `build_dictionary()` |
| `buildUserDictionary()` | `build_user_dictionary()` |

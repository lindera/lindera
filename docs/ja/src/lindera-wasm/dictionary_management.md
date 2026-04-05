# 辞書管理

## OPFS からの辞書読み込み

WASM で辞書を使用する推奨方法は、[GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードし、OPFS 経由で読み込むことです。これにより、WASM バイナリに大きな辞書を埋め込む必要がなくなります。

### バイトデータからの読み込み

OPFS やその他のブラウザストレージに保存された辞書を `loadDictionaryFromBytes()` で読み込みます。

#### `loadDictionaryFromBytes(metadata, dictDa, dictVals, dictWordsIdx, dictWords, matrixMtx, charDef, unk)`

- **パラメータ**:
  - `metadata` (`Uint8Array`) -- `metadata.json` の内容
  - `dictDa` (`Uint8Array`) -- `dict.da` の内容（Double-Array Trie）
  - `dictVals` (`Uint8Array`) -- `dict.vals` の内容（単語値データ）
  - `dictWordsIdx` (`Uint8Array`) -- `dict.wordsidx` の内容（単語詳細インデックス）
  - `dictWords` (`Uint8Array`) -- `dict.words` の内容（単語詳細）
  - `matrixMtx` (`Uint8Array`) -- `matrix.mtx` の内容（連接コスト行列）
  - `charDef` (`Uint8Array`) -- `char_def.bin` の内容（文字定義）
  - `unk` (`Uint8Array`) -- `unk.bin` の内容（未知語辞書）
- **戻り値**: `Dictionary`

```javascript
import { loadDictionaryFromBytes, TokenizerBuilder } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

// OPFS から辞書ファイルを読み込む
const files = await loadDictionaryFiles("ipadic");

// バイトデータから Dictionary を作成
const dictionary = loadDictionaryFromBytes(
    files.metadata,
    files.dictDa,
    files.dictVals,
    files.dictWordsIdx,
    files.dictWords,
    files.matrixMtx,
    files.charDef,
    files.unk,
);

// TokenizerBuilder で使用
const builder = new TokenizerBuilder();
builder.setDictionaryInstance(dictionary);
builder.setMode("normal");
const tokenizer = builder.build();
```

ダウンロードとキャッシュを含む完全なワークフローは [OPFS 辞書ストレージ](./opfs.md) を参照してください。

## 埋め込み辞書（上級者向け）

`embed-*` feature フラグ付きでビルドした場合、`embedded://` URI スキームで埋め込み辞書を読み込めます。WASM バイナリのサイズが大幅に増加します。

### 埋め込み辞書の読み込み

```javascript
import { loadDictionary } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
```

利用可能な埋め込み辞書の URI（ビルド時に有効にした feature に依存）：

| URI | Feature フラグ |
| --- | --- |
| `embedded://ipadic` | `embed-ipadic` |
| `embedded://unidic` | `embed-unidic` |
| `embedded://ko-dic` | `embed-ko-dic` |
| `embedded://cc-cedict` | `embed-cc-cedict` |
| `embedded://jieba` | `embed-jieba` |

### TokenizerBuilder での使用

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();
```

### Tokenizer コンストラクタでの使用

```javascript
import { loadDictionary, Tokenizer } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
const tokenizer = new Tokenizer(dictionary, "normal");
```

## Dictionary クラス

`Dictionary` クラスは、読み込み済みの形態素解析辞書を表します。

### プロパティ

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `name` | `string` | 辞書名（例: `"ipadic"`） |
| `encoding` | `string` | 辞書の文字エンコーディング |
| `metadata` | `Metadata` | 完全なメタデータオブジェクト |

```javascript
console.log(dictionary.name);     // "ipadic"
console.log(dictionary.encoding); // "utf-8"
```

## ユーザー辞書

ユーザー辞書を使用すると、システム辞書にないカスタム語彙を追加できます。

### ユーザー辞書の読み込み

```javascript
import { loadUserDictionary } from 'lindera-wasm-web';

const metadata = dictionary.metadata;
const userDict = loadUserDictionary("/path/to/user_dict.csv", metadata);
```

### Tokenizer でのユーザー辞書の使用

```javascript
import { loadDictionaryFromBytes, loadUserDictionary, Tokenizer } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);
const userDict = loadUserDictionary("/path/to/user_dict.csv", dictionary.metadata);
const tokenizer = new Tokenizer(dictionary, "normal", userDict);
```

### ユーザー辞書の CSV フォーマット

ユーザー辞書の CSV は Lindera ユーザー辞書と同じフォーマットに準拠します：

```csv
東京スカイツリー,カスタム名詞,トウキョウスカイツリー
東武スカイツリーライン,カスタム名詞,トウブスカイツリーライン
```

各行の構成: `surface,part_of_speech,reading`

## 辞書のビルド

JavaScript API を使用してソースファイルからコンパイル済み辞書をビルドできます。

### システム辞書のビルド

```javascript
import { buildDictionary } from 'lindera-wasm-web';

const metadata = {
    name: "custom-dict",
    encoding: "utf-8",
    // ... other metadata fields
};

buildDictionary("/path/to/source/dir", "/path/to/output/dir", metadata);
```

### ユーザー辞書のビルド

```javascript
import { buildUserDictionary } from 'lindera-wasm-web';

buildUserDictionary("/path/to/user_dict.csv", "/path/to/output/dir");
```

`buildUserDictionary` の `metadata` パラメータは省略可能です。省略した場合はデフォルトのメタデータが使用されます。

## Metadata

`Metadata` クラスは辞書のパラメータを設定します。

### コンストラクタ

```javascript
const metadata = new Metadata(name?, encoding?);
```

- **パラメータ**:
  - `name` (string, 省略可) -- 辞書名（デフォルト: `"default"`）
  - `encoding` (string, 省略可) -- 文字エンコーディング（デフォルト: `"UTF-8"`）

### 静的メソッド

#### `Metadata.createDefault()`

デフォルト値で `Metadata` インスタンスを作成します。

```javascript
const metadata = Metadata.createDefault();
```

### Metadata プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | 辞書名 |
| `encoding` | `string` | `"UTF-8"` | 文字エンコーディング |
| `dictionary_schema` | `Schema` | IPADIC スキーマ | メイン辞書のスキーマ |
| `user_dictionary_schema` | `Schema` | 最小スキーマ | ユーザー辞書のスキーマ |

すべてのプロパティは取得と設定の両方に対応しています：

```javascript
const metadata = Metadata.createDefault();
metadata.name = "custom_dict";
metadata.encoding = "EUC-JP";
console.log(metadata.name); // "custom_dict"
```

読み込み済み辞書のメタデータには `dictionary.metadata` からアクセスできます。

### Schema

`Schema` クラスは辞書エントリのフィールド構造を定義します。

#### Schema コンストラクタ

```javascript
const schema = new Schema(["surface", "left_id", "right_id", "cost", "pos", "reading"]);
```

#### Schema 静的メソッド

- `Schema.create_default()` -- デフォルトの IPADIC 互換スキーマを作成

#### Schema メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `get_field_index(name)` | `number \| undefined` | フィールド名からインデックスを取得 |
| `field_count()` | `number` | フィールドの総数 |
| `get_field_name(index)` | `string \| undefined` | インデックスからフィールド名を取得 |
| `get_custom_fields()` | `string[]` | インデックス 3 以降のフィールド（形態素素性） |
| `get_all_fields()` | `string[]` | すべてのフィールド名 |
| `get_field_by_name(name)` | `FieldDefinition \| undefined` | フィールド定義の完全な情報を取得 |

#### FieldDefinition

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `index` | `number` | フィールドの位置インデックス |
| `name` | `string` | フィールド名 |
| `field_type` | `FieldType` | フィールド型の列挙値 |
| `description` | `string \| undefined` | 説明（省略可） |

#### FieldType

| 値 | 説明 |
| --- | --- |
| `FieldType.Surface` | 単語の表層形テキスト |
| `FieldType.LeftContextId` | 左文脈 ID |
| `FieldType.RightContextId` | 右文脈 ID |
| `FieldType.Cost` | 単語コスト |
| `FieldType.Custom` | 形態素素性フィールド |

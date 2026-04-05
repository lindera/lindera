# 辞書管理

Lindera Node.js は、形態素解析で使用する辞書の読み込み、ビルド、管理のための関数を提供します。

## 辞書の読み込み

### システム辞書

`loadDictionary(uri)` を使用してシステム辞書を読み込みます。[GitHub Releases](https://github.com/lindera/lindera/releases) からビルド済み辞書をダウンロードし、展開したディレクトリのパスを指定してください：

```javascript
const { loadDictionary } = require("lindera-nodejs");

const dictionary = loadDictionary("/path/to/ipadic");
```

**埋め込み辞書（上級者向け）** -- `embed-*` feature フラグ付きでビルドした場合、埋め込み辞書を使用できます：

```javascript
const dictionary = loadDictionary("embedded://ipadic");
```

### ユーザー辞書

ユーザー辞書はシステム辞書にカスタム語彙を追加します。

```javascript
const { loadUserDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);
```

トークナイザーのビルド時にユーザー辞書を渡します：

```javascript
const { Tokenizer, loadDictionary, loadUserDictionary, Metadata } = require("lindera-nodejs");

const dictionary = loadDictionary("/path/to/ipadic");
const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);

const tokenizer = new Tokenizer(dictionary, "normal", userDict);
```

または、ビルダー経由で設定します：

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

const tokenizer = new TokenizerBuilder()
  .setDictionary("/path/to/ipadic")
  .setUserDictionary("/path/to/user_dictionary")
  .build();
```

## 辞書のビルド

### システム辞書のビルド

ソースファイルからシステム辞書をビルドします：

```javascript
const { buildDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata({ name: "custom", encoding: "UTF-8" });
buildDictionary("/path/to/input_dir", "/path/to/output_dir", metadata);
```

入力ディレクトリには辞書のソースファイル（CSV レキシコン、matrix.def など）が含まれている必要があります。

### ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします：

```javascript
const { buildUserDictionary, Metadata } = require("lindera-nodejs");

const metadata = new Metadata();
buildUserDictionary("ipadic", "user_words.csv", "/path/to/output_dir", metadata);
```

`metadata` パラメータは省略可能です。省略した場合はデフォルトのメタデータ値が使用されます：

```javascript
buildUserDictionary("ipadic", "user_words.csv", "/path/to/output_dir");
```

## Metadata

`Metadata` クラスは辞書のパラメータを設定します。

### Metadata の作成

```javascript
const { Metadata } = require("lindera-nodejs");

// デフォルトのメタデータ
const metadata = new Metadata();

// カスタムメタデータ
const metadata = new Metadata({
  name: "my_dictionary",
  encoding: "UTF-8",
  defaultWordCost: -10000,
});
```

### JSON からの読み込み

```javascript
const metadata = Metadata.fromJsonFile("metadata.json");
```

### プロパティ

| プロパティ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `name` | `string` | `"default"` | 辞書名 |
| `encoding` | `string` | `"UTF-8"` | 文字エンコーディング |
| `defaultWordCost` | `number` | `-10000` | 未知語のデフォルトコスト |
| `defaultLeftContextId` | `number` | `1288` | デフォルトの左文脈 ID |
| `defaultRightContextId` | `number` | `1288` | デフォルトの右文脈 ID |
| `defaultFieldValue` | `string` | `"*"` | 欠損フィールドのデフォルト値 |
| `flexibleCsv` | `boolean` | `false` | 柔軟な CSV パースを許可 |
| `skipInvalidCostOrId` | `boolean` | `false` | 無効なコストまたは ID のエントリーをスキップ |
| `normalizeDetails` | `boolean` | `false` | 形態素の詳細情報を正規化 |
| `dictionarySchema` | `Schema` | IPADIC スキーマ | メイン辞書のスキーマ |
| `userDictionarySchema` | `Schema` | 最小スキーマ | ユーザー辞書のスキーマ |

すべてのプロパティは取得と設定の両方をサポートしています：

```javascript
const metadata = new Metadata();
metadata.name = "custom_dict";
metadata.encoding = "EUC-JP";
console.log(metadata.name); // "custom_dict"
```

### `toObject()`

メタデータのオブジェクト表現を返します：

```javascript
const metadata = new Metadata({ name: "test" });
console.log(metadata.toObject());
```

## Schema

`Schema` クラスは辞書エントリーのフィールド構造を定義します。

### Schema の作成

```javascript
const { Schema } = require("lindera-nodejs");

// デフォルトの IPADIC 互換スキーマ
const schema = Schema.createDefault();

// カスタムスキーマ
const custom = new Schema(["surface", "left_id", "right_id", "cost", "pos", "reading"]);
```

### Schema メソッド

| メソッド | 戻り値 | 説明 |
| --- | --- | --- |
| `getFieldIndex(name)` | `number \| null` | フィールド名からインデックスを取得 |
| `fieldCount()` | `number` | フィールドの総数 |
| `getFieldName(index)` | `string \| null` | インデックスからフィールド名を取得 |
| `getCustomFields()` | `string[]` | インデックス 4 以降のフィールド（形態素素性） |
| `getAllFields()` | `string[]` | すべてのフィールド名 |
| `getFieldByName(name)` | `FieldDefinition \| null` | フィールド定義の完全な情報を取得 |
| `validateRecord(record)` | `void` | CSV レコードをスキーマに対して検証 |

```javascript
const schema = Schema.createDefault();

console.log(schema.fieldCount());           // 13（IPADIC フォーマット）
console.log(schema.getFieldIndex("pos1"));  // 例: 4
console.log(schema.getAllFields());          // ["surface", "left_id", ...]
console.log(schema.getCustomFields());      // インデックス 4 以降のフィールド
```

### FieldDefinition

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `index` | `number` | フィールドの位置インデックス |
| `name` | `string` | フィールド名 |
| `fieldType` | `FieldType` | フィールド型の列挙値 |
| `description` | `string \| undefined` | 任意の説明 |

### FieldType

| 値 | 説明 |
| --- | --- |
| `FieldType.Surface` | 単語の表層形 |
| `FieldType.LeftContextId` | 左文脈 ID |
| `FieldType.RightContextId` | 右文脈 ID |
| `FieldType.Cost` | 単語コスト |
| `FieldType.Custom` | 形態素素性フィールド |

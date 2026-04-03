# 辞書管理

Lindera Node.js は、形態素解析で使用する辞書の読み込み、ビルド、管理のための関数を提供します。

## 辞書の読み込み

### システム辞書

`loadDictionary(uri)` を使用してシステム辞書を読み込みます。

**埋め込み辞書**（対応する `embed-*` feature が必要）：

```javascript
const { loadDictionary } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
```

**外部辞書**（ディスク上のディレクトリから読み込み）：

```javascript
const dictionary = loadDictionary("/path/to/dictionary");
```

### ユーザー辞書

ユーザー辞書はシステム辞書にカスタム語彙を追加します。

```javascript
const { loadUserDictionary, Metadata } = require("lindera");

const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);
```

トークナイザーのビルド時にユーザー辞書を渡します：

```javascript
const { Tokenizer, loadDictionary, loadUserDictionary, Metadata } = require("lindera");

const dictionary = loadDictionary("embedded://ipadic");
const metadata = new Metadata();
const userDict = loadUserDictionary("/path/to/user_dictionary", metadata);

const tokenizer = new Tokenizer(dictionary, "normal", userDict);
```

または、ビルダー経由で設定します：

```javascript
const { TokenizerBuilder } = require("lindera");

const tokenizer = new TokenizerBuilder()
  .setDictionary("embedded://ipadic")
  .setUserDictionary("/path/to/user_dictionary")
  .build();
```

## 辞書のビルド

### システム辞書のビルド

ソースファイルからシステム辞書をビルドします：

```javascript
const { buildDictionary, Metadata } = require("lindera");

const metadata = new Metadata({ name: "custom", encoding: "UTF-8" });
buildDictionary("/path/to/input_dir", "/path/to/output_dir", metadata);
```

入力ディレクトリには辞書のソースファイル（CSV レキシコン、matrix.def など）が含まれている必要があります。

### ユーザー辞書のビルド

CSV ファイルからユーザー辞書をビルドします：

```javascript
const { buildUserDictionary, Metadata } = require("lindera");

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
const { Metadata, CompressionAlgorithm } = require("lindera");

// デフォルトのメタデータ
const metadata = new Metadata();

// カスタムメタデータ
const metadata = new Metadata({
  name: "my_dictionary",
  encoding: "UTF-8",
  compressAlgorithm: CompressionAlgorithm.Deflate,
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
| `compressAlgorithm` | `CompressionAlgorithm` | `Deflate` | 圧縮アルゴリズム |
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

### CompressionAlgorithm

利用可能な圧縮アルゴリズム：

| 値 | 説明 |
| --- | --- |
| `CompressionAlgorithm.Deflate` | DEFLATE 圧縮（デフォルト） |
| `CompressionAlgorithm.Zlib` | Zlib 圧縮 |
| `CompressionAlgorithm.Gzip` | Gzip 圧縮 |
| `CompressionAlgorithm.Raw` | 圧縮なし |

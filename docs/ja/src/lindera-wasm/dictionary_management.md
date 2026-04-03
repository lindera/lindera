# 辞書管理

## 埋め込み辞書

WASM で辞書を使用する最も簡単な方法は、feature フラグを使用してビルド時に辞書を埋め込むことです。埋め込み辞書は `embedded://` URI スキームで読み込みます。

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

`TokenizerBuilder` を使用する場合は、辞書の URI を直接設定します：

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();
```

### Tokenizer コンストラクタでの使用

読み込み済みの辞書を `Tokenizer` コンストラクタに渡すこともできます：

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
const dict = loadDictionary("embedded://ipadic");
console.log(dict.name);     // "ipadic"
console.log(dict.encoding); // "utf-8"
```

## ユーザー辞書

ユーザー辞書を使用すると、システム辞書にないカスタム語彙を追加できます。

### ユーザー辞書の読み込み

```javascript
import { loadUserDictionary } from 'lindera-wasm-web-ipadic';

const metadata = dictionary.metadata;
const userDict = loadUserDictionary("/path/to/user_dict.csv", metadata);
```

### Tokenizer でのユーザー辞書の使用

```javascript
import { loadDictionary, loadUserDictionary, Tokenizer } from 'lindera-wasm-web-ipadic';

const dictionary = loadDictionary("embedded://ipadic");
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
import { buildDictionary } from 'lindera-wasm-web-ipadic';

const metadata = {
    name: "custom-dict",
    encoding: "utf-8",
    // ... other metadata fields
};

buildDictionary("/path/to/source/dir", "/path/to/output/dir", metadata);
```

### ユーザー辞書のビルド

```javascript
import { buildUserDictionary } from 'lindera-wasm-web-ipadic';

buildUserDictionary("/path/to/user_dict.csv", "/path/to/output/dir");
```

`buildUserDictionary` の `metadata` パラメータは省略可能です。省略した場合はデフォルトのメタデータが使用されます。

## Metadata

`Metadata` オブジェクトには以下のような辞書設定が含まれます：

- 辞書名
- 文字エンコーディング
- 圧縮アルゴリズム
- 辞書フィールドのスキーマ定義

読み込み済み辞書のメタデータには `dictionary.metadata` からアクセスできます。

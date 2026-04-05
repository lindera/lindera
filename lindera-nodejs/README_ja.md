# lindera-nodejs

[Lindera](https://github.com/lindera/lindera) の Node.js バインディング。日本語形態素解析エンジン。

## 概要

lindera-nodejs は、Lindera 形態素解析エンジンへの包括的な Node.js インターフェースを提供し、日本語・韓国語・中国語のテキスト解析に対応しています。主な機能は以下の通りです:

- **多言語対応**: 日本語（IPADIC、IPADIC-NEologd、UniDic）、韓国語（ko-dic）、中国語（CC-CEDICT、Jieba）
- **文字フィルタ**: マッピング、正規表現、Unicode 正規化、日本語踊り字処理によるテキスト前処理
- **トークンフィルタ**: 小文字化、長さフィルタリング、ストップワード、日本語固有フィルタなどの後処理フィルタ
- **柔軟な設定**: トークナイズモードやペナルティ設定のカスタマイズ
- **メタデータ対応**: 辞書スキーマとメタデータの完全な管理
- **TypeScript 対応**: すぐに使える完全な型定義を同梱

## 機能

### コアコンポーネント

- **TokenizerBuilder**: カスタマイズされたトークナイザを構築するための Fluent API
- **Tokenizer**: フィルタリング機能を統合した高性能テキストトークナイズ
- **CharacterFilter**: テキスト正規化のための前処理フィルタ
- **TokenFilter**: トークン精緻化のための後処理フィルタ
- **Metadata & Schema**: 辞書構造と設定の管理
- **Training & Export**（オプション）: コーパスデータからカスタム形態素解析モデルを学習

### 対応辞書

- **日本語**: IPADIC、IPADIC-NEologd、UniDic
- **韓国語**: ko-dic
- **中国語**: CC-CEDICT、Jieba
- **カスタム**: ユーザー辞書対応

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) から入手できます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、展開したパスを指定して読み込みます。

### フィルタの種類

**文字フィルタ:**

- マッピングフィルタ（文字置換）
- 正規表現フィルタ（パターンベースの置換）
- Unicode 正規化（NFKC など）
- 日本語踊り字の正規化

**トークンフィルタ:**

- テキストの大文字・小文字変換
- 長さフィルタリング（最小/最大文字数）
- ストップワードフィルタリング
- 日本語固有フィルタ（基本形、読みなど）
- 韓国語固有フィルタ

## プロジェクト依存関係のインストール

- Node.js 18+ : <https://nodejs.org/>
- Rust : <https://www.rust-lang.org/tools/install>
- @napi-rs/cli : `npm install -g @napi-rs/cli`

## リポジトリのセットアップ

```shell
# Clone lindera project repository
git clone git@github.com:lindera/lindera.git
cd lindera
```

## lindera-nodejs のインストール

このコマンドは開発設定（デバッグビルド）でライブラリをビルドします。

```shell
cd lindera-nodejs
npm install
npm run build
```

## クイックスタート

### 基本的なトークナイズ

```javascript
const { loadDictionary, Tokenizer } = require("lindera-nodejs");

// Load dictionary
// Load dictionary from a local path (download from GitHub Releases)
const dictionary = loadDictionary("/path/to/ipadic");

// Create a tokenizer
const tokenizer = new Tokenizer(dictionary, "normal");

// Tokenize Japanese text
const text = "すもももももももものうち";
const tokens = tokenizer.tokenize(text);

for (const token of tokens) {
  console.log(`Text: ${token.surface}, Position: ${token.byteStart}-${token.byteEnd}`);
}
```

### 文字フィルタの使用

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

// Create tokenizer builder
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("/path/to/ipadic");

// Add character filters
builder.appendCharacterFilter("mapping", { mapping: { "ー": "-" } });
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });

// Build tokenizer with filters
const tokenizer = builder.build();
const text = "テストー１２３";
const tokens = tokenizer.tokenize(text); // Will apply filters automatically
```

### トークンフィルタの使用

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

// Create tokenizer builder
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("/path/to/ipadic");

// Add token filters
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("length", { min: 2, max: 10 });
builder.appendTokenFilter("japanese_stop_tags", { tags: ["助詞", "助動詞"] });

// Build tokenizer with filters
const tokenizer = builder.build();
const tokens = tokenizer.tokenize("テキストの解析");
```

### 統合パイプライン

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

// Build tokenizer with integrated filters
const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("/path/to/ipadic");

// Add character filters
builder.appendCharacterFilter("mapping", { mapping: { "ー": "-" } });
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });

// Add token filters
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("japanese_base_form");

// Build and use
const tokenizer = builder.build();
const tokens = tokenizer.tokenize("コーヒーショップ");
```

### メタデータの操作

```javascript
const { Metadata } = require("lindera-nodejs");

// Create metadata with default values
const metadata = new Metadata();
console.log(`Name: ${metadata.name}`);
console.log(`Encoding: ${metadata.encoding}`);

// Create metadata from a JSON file
const loaded = Metadata.fromJsonFile("metadata.json");
console.log(loaded.toObject());
```

## 応用的な使い方

### フィルタ設定の例

文字フィルタとトークンフィルタは、オブジェクト型の引数で設定を受け取ります:

```javascript
const { TokenizerBuilder } = require("lindera-nodejs");

const builder = new TokenizerBuilder();
builder.setDictionary("/path/to/ipadic");

// Character filters with object configuration
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
builder.appendCharacterFilter("japanese_iteration_mark", {
  normalize_kanji: true,
  normalize_kana: true,
});
builder.appendCharacterFilter("mapping", {
  mapping: { "リンデラ": "lindera", "トウキョウ": "東京" },
});

// Token filters with object configuration
builder.appendTokenFilter("japanese_katakana_stem", { min: 3 });
builder.appendTokenFilter("length", { min: 2, max: 10 });
builder.appendTokenFilter("japanese_stop_tags", {
  tags: ["助詞", "助動詞", "記号"],
});

// Filters without configuration can omit the object
builder.appendTokenFilter("lowercase");
builder.appendTokenFilter("japanese_base_form");

const tokenizer = builder.build();
```

`examples/` ディレクトリに包括的な使用例があります:

- `tokenize.js`: 基本的なトークナイズ
- `tokenize_with_filters.js`: 文字フィルタとトークンフィルタの使用
- `tokenize_with_userdict.js`: カスタムユーザー辞書
- `train_and_export.js`: カスタム辞書の学習とエクスポート（`train` feature が必要）
- `tokenize_with_decompose.js`: Decompose モードのトークナイズ

## 辞書サポート

### 日本語

- **IPADIC**: 標準的な日本語辞書。一般的なテキストに適しています
- **UniDic**: 詳細な形態素情報を持つ学術辞書

### 韓国語

- **ko-dic**: 形態素解析のための標準韓国語辞書

### 中国語

- **CC-CEDICT**: コミュニティが管理する中国語-英語辞書

### カスタム辞書

- ドメイン固有の用語に対応するユーザー辞書サポート
- CSV 形式で簡単にカスタマイズ可能

## 辞書の学習（実験的）

lindera-nodejs は、`train` feature を有効にしてビルドすることで、アノテーション済みコーパスデータからカスタム形態素解析モデルの学習をサポートします。

### 学習サポート付きでビルド

```shell
npm run build -- --features train
```

### モデルの学習

```javascript
const { train } = require("lindera-nodejs");

// Train a model from corpus
train({
  seed: "path/to/seed.csv",
  corpus: "path/to/corpus.txt",
  charDef: "path/to/char.def",
  unkDef: "path/to/unk.def",
  featureDef: "path/to/feature.def",
  rewriteDef: "path/to/rewrite.def",
  output: "model.dat",
  lambda: 0.01,
  maxIter: 100,
});
```

### 辞書ファイルのエクスポート

```javascript
const { exportModel } = require("lindera-nodejs");

// Export trained model to dictionary files
exportModel({
  model: "model.dat",
  output: "exported_dict/",
  metadata: "metadata.json",
});
```

以下のファイルが生成されます:

- `lex.csv`: 語彙ファイル
- `matrix.def`: 連接コスト行列
- `unk.def`: 未知語定義
- `char.def`: 文字定義
- `metadata.json`: 辞書メタデータ（指定した場合）

完全な使用例は `examples/train_and_export.js` を参照してください。

## API リファレンス

### コアクラス

- `TokenizerBuilder`: トークナイザ設定のための Fluent ビルダー
- `Tokenizer`: メインのトークナイズエンジン
- `Token`: テキスト、位置、言語的特徴を持つ個々のトークン
- `Metadata`: 辞書メタデータと設定
- `Schema`: 辞書スキーマ定義

### 学習関数（`train` feature が必要）

- `train()`: コーパスから形態素解析モデルを学習
- `exportModel()`: 学習済みモデルを辞書ファイルにエクスポート

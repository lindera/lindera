# 学習

Lindera Node.js は、アノテーション付きコーパスからカスタム CRF ベースの形態素解析モデルを学習する機能をサポートしています。この機能には `train` feature が必要です。

## 前提条件

`train` feature を有効にして lindera-nodejs をビルドします（デフォルトで有効）：

```bash
npm run build -- --features train
```

## モデルの学習

`train()` を使用して、種辞書とアノテーション付きコーパスから CRF モデルを学習します：

```javascript
const { train } = require("lindera-nodejs");

train({
  seed: "resources/training/seed.csv",
  corpus: "resources/training/corpus.txt",
  charDef: "resources/training/char.def",
  unkDef: "resources/training/unk.def",
  featureDef: "resources/training/feature.def",
  rewriteDef: "resources/training/rewrite.def",
  output: "/tmp/model.dat",
  lambda: 0.01,
  maxIter: 100,
  maxThreads: 4,
});
```

### 学習パラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `seed` | `string` | 必須 | 種辞書ファイルのパス（CSV 形式） |
| `corpus` | `string` | 必須 | アノテーション付き学習コーパスのパス |
| `charDef` | `string` | 必須 | 文字定義ファイルのパス（char.def） |
| `unkDef` | `string` | 必須 | 未知語定義ファイルのパス（unk.def） |
| `featureDef` | `string` | 必須 | 素性定義ファイルのパス（feature.def） |
| `rewriteDef` | `string` | 必須 | 書き換えルール定義ファイルのパス（rewrite.def） |
| `output` | `string` | 必須 | 学習済みモデルファイルの出力パス |
| `lambda` | `number` | `0.01` | L1 正則化コスト（0.0--1.0） |
| `maxIter` | `number` | `100` | 最大学習イテレーション数 |
| `maxThreads` | `number \| undefined` | `undefined` | スレッド数（undefined = CPU コア数を自動検出） |

## 学習済みモデルのエクスポート

学習後、`exportModel()` を使用してモデルを辞書ソースファイルにエクスポートします：

```javascript
const { exportModel } = require("lindera-nodejs");

exportModel({
  model: "/tmp/model.dat",
  output: "/tmp/dictionary_source",
  metadata: "resources/training/metadata.json",
});
```

### エクスポートパラメータ

| パラメータ | 型 | デフォルト | 説明 |
| --- | --- | --- | --- |
| `model` | `string` | 必須 | 学習済みモデルファイルのパス（.dat） |
| `output` | `string` | 必須 | 辞書ソースファイルの出力ディレクトリ |
| `metadata` | `string \| undefined` | `undefined` | ベースとなる metadata.json ファイルのパス |

エクスポートにより、出力ディレクトリに以下のファイルが作成されます：

- `lex.csv` -- 学習済みコスト付きのレキシコンエントリー
- `matrix.def` -- 連接コスト行列
- `unk.def` -- 未知語定義
- `char.def` -- 文字カテゴリ定義
- `metadata.json` -- 更新されたメタデータ（`metadata` パラメータ指定時）

## 完全なワークフロー

カスタム辞書の学習と使用の完全なワークフロー：

```javascript
const {
  train,
  exportModel,
  buildDictionary,
  Metadata,
  TokenizerBuilder,
} = require("lindera-nodejs");

// Step 1: Train the CRF model
train({
  seed: "resources/training/seed.csv",
  corpus: "resources/training/corpus.txt",
  charDef: "resources/training/char.def",
  unkDef: "resources/training/unk.def",
  featureDef: "resources/training/feature.def",
  rewriteDef: "resources/training/rewrite.def",
  output: "/tmp/model.dat",
  lambda: 0.01,
  maxIter: 100,
});

// Step 2: Export to dictionary source files
exportModel({
  model: "/tmp/model.dat",
  output: "/tmp/dictionary_source",
  metadata: "resources/training/metadata.json",
});

// Step 3: Build the dictionary from exported source files
const metadata = Metadata.fromJsonFile("/tmp/dictionary_source/metadata.json");
buildDictionary("/tmp/dictionary_source", "/tmp/dictionary", metadata);

// Step 4: Use the trained dictionary
const tokenizer = new TokenizerBuilder()
  .setDictionary("/tmp/dictionary")
  .setMode("normal")
  .build();

const tokens = tokenizer.tokenize("形態素解析のテスト");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.details.join(",")}`);
}
```

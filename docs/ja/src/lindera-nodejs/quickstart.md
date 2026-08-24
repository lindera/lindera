# クイックスタート

このガイドでは、lindera-nodejs を使用してテキストをトークナイズする方法を紹介します。

## 基本的なトークナイズ

トークナイザーの作成には `TokenizerBuilder` の使用を推奨します：

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("/path/to/ipadic");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("関西国際空港限定トートバッグ");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.details.join(",")}`);
}
```

> **注意:** ビルド済み辞書を [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードし、展開したディレクトリのパスを指定してください。

期待される出力：

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## トークンプロパティへのアクセス

各トークンは以下のプロパティを公開しています：

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setDictionary("/path/to/ipadic");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("東京タワー");
for (const token of tokens) {
  console.log(`Surface: ${token.surface}`);
  console.log(`Byte range: ${token.byteStart}..${token.byteEnd}`);
  console.log(`Position: ${token.position}`);
  console.log(`Word ID: ${token.wordId}`);
  console.log(`Unknown: ${token.isUnknown}`);
  console.log(`Details: ${token.details}`);
  console.log();
}
```

## N-best トークナイズ

コスト順にランク付けされた複数のトークナイズ候補を取得します：

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setDictionary("/path/to/ipadic");
const tokenizer = builder.build();

const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const { tokens, cost } of results) {
  const surfaces = tokens.map((t) => t.surface);
  console.log(`Cost ${cost}: ${surfaces.join(" / ")}`);
}
```

## TypeScript

Lindera Node.js には TypeScript の型定義が同梱されています。すべてのクラスと関数に完全な型が付いています。npm パッケージ `lindera` の `exports` マップは `require` / `import` の両方の条件を宣言しているため、CommonJS からも ES モジュールからもそのまま読み込めます。以下のサンプルは `moduleResolution: node16` + `strict` でコンパイルできることを確認済みです：

```typescript
import type { Token } from "lindera";
import { TokenizerBuilder } from "lindera";

const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("/path/to/ipadic");
const tokenizer = builder.build();

const tokens: Token[] = tokenizer.tokenize("形態素解析");
for (const token of tokens) {
  console.log(`${token.surface}: ${token.details.join(",")}`);
}
```

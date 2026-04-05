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
トートバッグ    UNK
```

## メソッドチェーン

`TokenizerBuilder` は簡潔な設定のためにメソッドチェーンをサポートしています：

```javascript
const { TokenizerBuilder } = require("lindera");

const tokenizer = new TokenizerBuilder()
  .setMode("normal")
  .setDictionary("/path/to/ipadic")
  .build();

const tokens = tokenizer.tokenize("すもももももももものうち");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.getDetail(0)}`);
}
```

## トークンプロパティへのアクセス

各トークンは以下のプロパティを公開しています：

```javascript
const { TokenizerBuilder } = require("lindera");

const tokenizer = new TokenizerBuilder()
  .setDictionary("/path/to/ipadic")
  .build();

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

const tokenizer = new TokenizerBuilder()
  .setDictionary("/path/to/ipadic")
  .build();

const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const { tokens, cost } of results) {
  const surfaces = tokens.map((t) => t.surface);
  console.log(`Cost ${cost}: ${surfaces.join(" / ")}`);
}
```

## TypeScript

Lindera Node.js には TypeScript の型定義が含まれています。すべてのクラスと関数に完全な型が付いています：

```typescript
import { TokenizerBuilder, Token } from "lindera";

const tokenizer = new TokenizerBuilder()
  .setMode("normal")
  .setDictionary("/path/to/ipadic")
  .build();

const tokens: Token[] = tokenizer.tokenize("形態素解析");
for (const token of tokens) {
  console.log(`${token.surface}: ${token.details?.join(",")}`);
}
```

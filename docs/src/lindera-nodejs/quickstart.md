# Quick Start

This guide shows how to tokenize text using lindera-nodejs.

## Basic Tokenization

The recommended way to create a tokenizer is through `TokenizerBuilder`:

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

> **Note:** Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases) and specify the path to the extracted directory.

Expected output:

```text
関西国際空港    名詞,固有名詞,組織,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    UNK
```

## Method Chaining

`TokenizerBuilder` supports method chaining for concise configuration:

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

## Accessing Token Properties

Each token exposes the following properties:

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

## N-best Tokenization

Retrieve multiple tokenization candidates ranked by cost:

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

Lindera Node.js includes TypeScript type definitions. All classes and functions are fully typed:

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

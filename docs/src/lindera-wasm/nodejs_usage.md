# Node.js Usage

## CommonJS (require)

When built with `--target nodejs`, the package can be loaded with `require()`:

```javascript
const { TokenizerBuilder } = require('lindera-wasm-ipadic-nodejs');

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("東京スカイツリー");
tokens.forEach(token => {
    console.log(`${token.surface}\t${token.details.join(',')}`);
});
```

No explicit WASM initialization is needed with the Node.js target -- the module handles it automatically.

## ES Modules (import)

If your Node.js project uses ES Modules (with `"type": "module"` in `package.json` or `.mjs` files):

```javascript
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { TokenizerBuilder } = require('lindera-wasm-ipadic-nodejs');

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("形態素解析");
for (const token of tokens) {
    console.log(`${token.surface}\t${token.details.join(',')}`);
}
```

## Server-Side Tokenization Example

A simple Express.js endpoint that tokenizes input text:

```javascript
const express = require('express');
const { TokenizerBuilder } = require('lindera-wasm-ipadic-nodejs');

const app = express();
app.use(express.json());

// Create the tokenizer once at startup
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();

app.post('/tokenize', (req, res) => {
    const { text } = req.body;
    if (!text) {
        return res.status(400).json({ error: 'Missing "text" field' });
    }

    const tokens = tokenizer.tokenize(text);
    const result = tokens.map(token => ({
        surface: token.surface,
        details: token.details,
        byteStart: token.byteStart,
        byteEnd: token.byteEnd,
    }));

    res.json({ tokens: result });
});

app.listen(3000, () => {
    console.log('Tokenization server running on port 3000');
});
```

### Example Request

```bash
curl -X POST http://localhost:3000/tokenize \
  -H "Content-Type: application/json" \
  -d '{"text": "関西国際空港限定トートバッグ"}'
```

## Using Multiple Dictionaries

If your application needs to tokenize multiple languages, you can create separate tokenizers:

```javascript
const { TokenizerBuilder } = require('lindera-wasm-cjk-nodejs');

// Japanese tokenizer
const jaBuilder = new TokenizerBuilder();
jaBuilder.setDictionary("embedded://ipadic");
jaBuilder.setMode("normal");
const jaTokenizer = jaBuilder.build();

// Korean tokenizer
const koBuilder = new TokenizerBuilder();
koBuilder.setDictionary("embedded://ko-dic");
koBuilder.setMode("normal");
const koTokenizer = koBuilder.build();

console.log(jaTokenizer.tokenize("東京タワー"));
console.log(koTokenizer.tokenize("서울타워"));
```

This requires building with the `embed-cjk` feature flag, which includes IPADIC, ko-dic, and Jieba dictionaries.

## N-Best Tokenization

Retrieve multiple candidate tokenizations on the server:

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const result of results) {
    console.log(`Cost: ${result.cost}`);
    for (const token of result.tokens) {
        console.log(`  ${token.surface}\t${token.details.join(',')}`);
    }
}
```

# Node.js での使用

## CommonJS (require)

`--target nodejs` でビルドした場合、パッケージは `require()` で読み込めます：

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

Node.js ターゲットでは、明示的な WASM 初期化は不要です。モジュールが自動的に処理します。

## ES Modules (import)

Node.js プロジェクトで ES Modules を使用している場合（`package.json` に `"type": "module"` を指定、または `.mjs` ファイル）：

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

## サーバーサイドトークナイズの例

入力テキストをトークナイズする簡単な Express.js エンドポイント：

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

### リクエスト例

```bash
curl -X POST http://localhost:3000/tokenize \
  -H "Content-Type: application/json" \
  -d '{"text": "関西国際空港限定トートバッグ"}'
```

## 複数辞書の使用

アプリケーションで複数言語のトークナイズが必要な場合は、個別のトークナイザーを作成できます：

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

この例では `embed-cjk` feature フラグを使用してビルドする必要があります。これにより IPADIC、ko-dic、Jieba の辞書が含まれます。

## N-Best トークナイズ

サーバー上で複数のトークナイズ候補を取得します：

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
for (const result of results) {
    console.log(`Cost: ${result.cost}`);
    for (const token of result.tokens) {
        console.log(`  ${token.surface}\t${token.details.join(',')}`);
    }
}
```

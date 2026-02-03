# lindera-wasm

WebAssembly of Lindera

![Screenshot from 2025-09-13 23-05-49](https://github.com/user-attachments/assets/a6ca165a-825c-4260-ba52-d76cd262a21f)

## Demo Application

- <https://lindera.github.io/lindera/demo/>

## npm

### For Web

- <https://www.npmjs.com/package/lindera-wasm-web>
Lindera WASM without a dictionary for Web

- <https://www.npmjs.com/package/lindera-wasm-cjk-web>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Web

- <https://www.npmjs.com/package/lindera-wasm-ipadic-web>  
Lindera WASM with Japanese dictionary (IPADIC) for Web

- <https://www.npmjs.com/package/lindera-wasm-unidic-web>  
Lindera WASM with Japanese dictionary (UniDic) for Web

- <https://www.npmjs.com/package/lindera-wasm-ko-dic-web>  
Lindera WASM with Korean dictionary (ko-dic) for Web

- <https://www.npmjs.com/package/lindera-wasm-cc-cedict-web>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Web

### For Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs>
Lindera WASM without a dictionary for Node.js

- <https://www.npmjs.com/package/lindera-wasm-cjk-nodejs>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-ipadic-nodejs>  
Lindera WASM with Japanese dictionary (IPADIC) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-unidic-nodejs>  
Lindera WASM with Japanese dictionary (UniDic) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-ko-dic-nodejs>  
Lindera WASM with Korean dictionary (ko-dic) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-cc-cedict-nodejs>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Node.js

### For bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler>
Lindera WASM without a dictionary for Bundler

- <https://www.npmjs.com/package/lindera-wasm-cjk-bundler>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-ipadic-bundler>  
Lindera WASM with Japanese dictionary (IPADIC) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-unidic-bundler>  
Lindera WASM with Japanese dictionary (UniDic) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-ko-dic-bundler>  
Lindera WASM with Korean dictionary (ko-dic) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-cc-cedict-bundler>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Bundler

## Usage

### Web Usage

Use the `-web` packages for browser environments with `<script type="module">`:

```html
<script type="module">
import __wbg_init, { TokenizerBuilder } from 'https://cdn.jsdelivr.net/npm/lindera-wasm-ipadic-web/lindera_wasm.js';

__wbg_init().then(() => {
    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("すもももももももものうち");
    tokens.forEach(token => {
        console.log(`${token.surface}: ${token.details.join(", ")}`);
    });
});
</script>
```

Or with a bundler:

```js
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-ipadic-web';

async function main() {
    await __wbg_init();

    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("すもももももももものうち");
    tokens.forEach(token => {
        console.log(`${token.surface}: ${token.details.join(", ")}`);
    });
}

main();
```

### Node.js Usage

Use the `-nodejs` packages for Node.js environments:

```js
const { TokenizerBuilder } = require('lindera-wasm-ipadic-nodejs');

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("すもももももももものうち");
tokens.forEach(token => {
    console.log(`${token.surface}: ${token.details.join(", ")}`);
});
```

Or with ESM:

```js
import { TokenizerBuilder } from 'lindera-wasm-ipadic-nodejs';

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("すもももももももものうち");
tokens.forEach(token => {
    console.log(`${token.surface}: ${token.details.join(", ")}`);
});
```

### Bundler Usage (Webpack, Rollup, etc.)

Use the `-bundler` packages for bundler environments:

```js
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-ipadic-bundler';

async function main() {
    await __wbg_init();

    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("すもももももももものうち");
    tokens.forEach(token => {
        console.log(`${token.surface}: ${token.details.join(", ")}`);
    });
}

main();
```

### Token Properties

Each token object has the following properties:

| Property | Type | Description |
| -------- | ---- | ----------- |
| `surface` | `string` | Surface form of the token |
| `byteStart` | `number` | Start byte position in the original text |
| `byteEnd` | `number` | End byte position in the original text |
| `position` | `number` | Position index of the token |
| `wordId` | `number` | Word ID in the dictionary |
| `details` | `string[]` | Morphological details array |

Methods:

- `getDetail(index)`: Returns the detail at the specified index, or `undefined` if not found
- `toJSON()`: Returns the token as a plain JavaScript object

### For Vite Projects

You should exclude this package in the `optimizeDeps`:

```js
// vite.config.js
import { defineConfig } from 'vite'

export default defineConfig({
  optimizeDeps: {
    exclude: [
      "lindera-wasm-ipadic-web"
    ]
  },
})
```

### For Browser Extension Development

Set the `cors` config in vite.config.js:

```js
// vite.config.js
import { defineConfig } from 'vite'

export default defineConfig({
  server: {
    cors: {
      origin: [
        /chrome-extension:\/\//,
      ],
    },
  },
})
```

And set the `content_security_policy` to contain `wasm-unsafe-eval` in manifest.json:

```json
"content_security_policy": {
  "extension_pages": "script-src 'self' 'wasm-unsafe-eval';"
}
```

## Development

### Install project dependencies

- wasm-pack : <https://rustwasm.github.io/wasm-pack/installer/>

### Setup repository

```shell
# Clone the Lindera project repository
% git clone git@github.com:lindera/lindera.git
% cd lindera
```

### Build project

```shell
% make wasm-build
```

### Run tests

```shell
% make wasm-test
```

### Build example web application

```shell
% cd example && npm install && npm run build && cp index.html dist/index.html
```

### Run example web application

```shell
% cd example && npm run start
```

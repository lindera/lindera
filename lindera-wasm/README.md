# lindera-wasm

WebAssembly of Lindera

![Screenshot from 2025-09-13 23-05-49](https://github.com/user-attachments/assets/a6ca165a-825c-4260-ba52-d76cd262a21f)

## Demo Application

- <https://lindera.github.io/lindera/demo/>

## npm

### For Web

- <https://www.npmjs.com/package/lindera-wasm-web>
Lindera WASM without a dictionary for Web

- <https://www.npmjs.com/package/lindera-wasm-web-cjk>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Web

- <https://www.npmjs.com/package/lindera-wasm-web-ipadic>  
Lindera WASM with Japanese dictionary (IPADIC) for Web

- <https://www.npmjs.com/package/lindera-wasm-web-unidic>  
Lindera WASM with Japanese dictionary (UniDic) for Web

- <https://www.npmjs.com/package/lindera-wasm-web-ko-dic>  
Lindera WASM with Korean dictionary (ko-dic) for Web

- <https://www.npmjs.com/package/lindera-wasm-web-cc-cedict>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Web

- <https://www.npmjs.com/package/lindera-wasm-web-jieba>  
Lindera WASM with Chinese dictionary (Jieba) for Web

### For Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler>
Lindera WASM without a dictionary for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-cjk>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-ipadic>  
Lindera WASM with Japanese dictionary (IPADIC) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-unidic>  
Lindera WASM with Japanese dictionary (UniDic) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-ko-dic>  
Lindera WASM with Korean dictionary (ko-dic) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-cc-cedict>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Bundler

- <https://www.npmjs.com/package/lindera-wasm-bundler-jieba>  
Lindera WASM with Chinese dictionary (Jieba) for Bundler

## Usage

### Web Usage

Use the `-web` packages for browser environments with `<script type="module">`:

```html
<script type="module">
import __wbg_init, { TokenizerBuilder } from 'https://cdn.jsdelivr.net/npm/lindera-wasm-web-ipadic/lindera_wasm.js';

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
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-web-ipadic';

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

### Bundler Usage (Webpack, Rollup, etc.)

Use the `-bundler` packages for bundler environments:

```js
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-bundler-ipadic';

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
      "lindera-wasm-web-ipadic"
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

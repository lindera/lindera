# lindera-wasm

WebAssembly of Lindera

![Screenshot from 2025-09-13 23-05-49](https://github.com/user-attachments/assets/a6ca165a-825c-4260-ba52-d76cd262a21f)

## Demo Application

- <https://lindera.github.io/lindera/demo/>

## npm

- <https://www.npmjs.com/package/lindera-wasm-web> — Lindera WASM for Web
- <https://www.npmjs.com/package/lindera-wasm-bundler> — Lindera WASM for Bundler

## Dictionary

Pre-built dictionaries are available from [GitHub Releases](https://github.com/lindera/lindera/releases).
Download a dictionary archive (e.g. `lindera-ipadic-*.zip`) and load it at runtime using the OPFS API.
See the [example application](example/) for a working demo of downloading and loading dictionaries.

## Usage

### Web Usage

Use the `lindera-wasm-web` package for browser environments.
Dictionaries are loaded at runtime from a local path or downloaded from [GitHub Releases](https://github.com/lindera/lindera/releases) using the OPFS API.

```js
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    // Download dictionary from GitHub Releases (first time only)
    await downloadDictionary("https://github.com/lindera/lindera/releases/download/v3.0.0/lindera-ipadic-3.0.0.zip", "ipadic");

    // Load dictionary from OPFS
    const files = await loadDictionaryFiles("ipadic");
    const dict = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals,
        files.dictWordsIdx, files.dictWords, files.matrixMtx,
        files.charDef, files.unk
    );

    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dict);
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

Use the `lindera-wasm-bundler` package for bundler environments.
The dictionary loading approach is the same as the web usage above.

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
      "lindera-wasm-web"
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

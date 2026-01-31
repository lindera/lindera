# lindera-wasm

WebAssembly of Lindera

![Screenshot from 2025-09-13 23-05-49](https://github.com/user-attachments/assets/a6ca165a-825c-4260-ba52-d76cd262a21f)

## Demo Application

- <https://lindera.github.io/lindera-wasm/>

## npm

### Web

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

### Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs-cjk>  
Lindera WASM with CJK dictionaries (IPADIC, ko-dic, CC-CEDICT) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs-ipadic>  
Lindera WASM with Japanese dictionary (IPADIC) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs-unidic>  
Lindera WASM with Japanese dictionary (UniDic) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs-ko-dic>  
Lindera WASM with Korean dictionary (ko-dic) for Node.js

- <https://www.npmjs.com/package/lindera-wasm-nodejs-cc-cedict>  
Lindera WASM with Chinese dictionary (CC-CEDICT) for Node.js

## Usage

init the wasm module before construct `TokenizerBuilder`:

```ts
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm'

__wbg_init.then(() => {
    const builder = new TokenizerBuilder()
    //...
})
```

### for [Vite](https://vite.dev/) base project

You should exclude this package in the `optimizeDeps`:

```ts
// vite.config.js
import { defineConfig } from 'vite'

export default defineConfig({
  optimizeDeps: {
    exclude: [
      "lindera-wasm"
    ]
  },
})
```

### for Browser extension development

Set the `cors` config in vite.config.js

```ts
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

and set the `content_security_policy` to contains `wasm-unsafe-eval` in manifest.json:

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

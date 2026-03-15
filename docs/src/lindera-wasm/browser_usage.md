# Browser Usage

## ES Module Import

In browser environments, you must initialize the WASM module before using any Lindera functions. The default export `__wbg_init` handles this initialization.

```javascript
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-ipadic-web';

async function main() {
    // Initialize the WASM module (must be called once before using any API)
    await __wbg_init();

    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("形態素解析を行います");
    tokens.forEach(token => {
        console.log(`${token.surface}: ${token.details.join(',')}`);
    });
}

main();
```

## HTML Example

A minimal HTML page using lindera-wasm:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Lindera WASM Demo</title>
</head>
<body>
    <textarea id="input" rows="4" cols="50">関西国際空港限定トートバッグ</textarea>
    <br>
    <button id="tokenize">Tokenize</button>
    <pre id="output"></pre>

    <script type="module">
        import __wbg_init, { TokenizerBuilder } from './pkg/lindera_wasm.js';

        let tokenizer;

        async function init() {
            await __wbg_init();
            const builder = new TokenizerBuilder();
            builder.setDictionary("embedded://ipadic");
            builder.setMode("normal");
            tokenizer = builder.build();
        }

        document.getElementById('tokenize').addEventListener('click', () => {
            const text = document.getElementById('input').value;
            const tokens = tokenizer.tokenize(text);
            const output = tokens.map(t =>
                `${t.surface}\t${t.details.join(',')}`
            ).join('\n');
            document.getElementById('output').textContent = output;
        });

        init();
    </script>
</body>
</html>
```

## Webpack Configuration

When using Webpack 5, enable the `asyncWebAssembly` experiment:

```javascript
// webpack.config.js
module.exports = {
    experiments: {
        asyncWebAssembly: true,
    },
    module: {
        rules: [
            {
                test: /\.wasm$/,
                type: "webassembly/async",
            },
        ],
    },
};
```

Then import using the bundler target build:

```javascript
import { TokenizerBuilder } from 'lindera-wasm-ipadic-bundler';

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.setMode("normal");
const tokenizer = builder.build();
```

With the bundler target, `__wbg_init()` is called automatically by the bundler.

## Vite / Rollup Setup

Vite supports WASM out of the box with the web target. Place the built `pkg/` directory in your project and import directly:

```javascript
import __wbg_init, { TokenizerBuilder } from './pkg/lindera_wasm.js';

await __wbg_init();
// ... use TokenizerBuilder as normal
```

For the bundler target with Vite, you may need the [vite-plugin-wasm](https://github.com/nicolo-ribaudo/vite-plugin-wasm) plugin:

```javascript
// vite.config.js
import wasm from 'vite-plugin-wasm';

export default {
    plugins: [wasm()],
};
```

## Chrome Extension Considerations

Chrome extensions using Manifest V3 restrict `WebAssembly.compile` and `WebAssembly.instantiate` by default. To use lindera-wasm in an extension, you need to add `wasm-unsafe-eval` to your Content Security Policy:

```json
{
    "content_security_policy": {
        "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'"
    }
}
```

Note that `wasm-unsafe-eval` only allows WebAssembly execution and does not permit arbitrary JavaScript `eval()`.

## Performance Tips

- **Initialize once**: Call `__wbg_init()` once at application startup, not on every tokenization request.
- **Reuse the tokenizer**: Create the `Tokenizer` instance once and reuse it for multiple calls to `tokenize()`.
- **Web Workers**: For heavy tokenization workloads, consider running Lindera in a Web Worker to avoid blocking the main thread.

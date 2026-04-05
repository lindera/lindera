# ブラウザでの使用

## ES Module インポート

ブラウザ環境では、Lindera の関数を使用する前に WASM モジュールを初期化する必要があります。デフォルトエクスポートの `__wbg_init` がこの初期化を処理します。

推奨される方法は、辞書を WASM バイナリに埋め込むのではなく、OPFS から読み込むことです：

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles, hasDictionary } from 'lindera-wasm-web/opfs';

async function main() {
    // Initialize the WASM module (must be called once before using any API)
    await __wbg_init();

    // キャッシュされていない場合は辞書をダウンロード
    if (!await hasDictionary("ipadic")) {
        await downloadDictionary(
            "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
            "ipadic",
        );
    }

    // OPFS から辞書を読み込み
    const files = await loadDictionaryFiles("ipadic");
    const dictionary = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
        files.dictWords, files.matrixMtx, files.charDef, files.unk,
    );

    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dictionary);
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("形態素解析を行います");
    tokens.forEach(token => {
        console.log(`${token.surface}: ${token.details.join(',')}`);
    });
}

main();
```

## 埋め込み辞書の使用（上級者向け）

`embed-*` feature フラグ付きでビルドした場合、OPFS の代わりに埋め込み辞書を使用できます：

```javascript
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-web-ipadic';

async function main() {
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

## HTML の例

OPFS 辞書ロードを使用した lindera-wasm の最小限の HTML ページ：

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
    <button id="tokenize" disabled>Tokenize</button>
    <pre id="output">Loading dictionary...</pre>

    <script type="module">
        import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from './pkg/lindera_wasm.js';
        import { downloadDictionary, loadDictionaryFiles, hasDictionary } from './pkg/opfs.js';

        let tokenizer;

        async function init() {
            await __wbg_init();

            // キャッシュされていない場合は辞書をダウンロード
            if (!await hasDictionary("ipadic")) {
                document.getElementById('output').textContent = 'Downloading dictionary...';
                await downloadDictionary(
                    "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
                    "ipadic",
                );
            }

            // OPFS から辞書を読み込み
            const files = await loadDictionaryFiles("ipadic");
            const dictionary = loadDictionaryFromBytes(
                files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
                files.dictWords, files.matrixMtx, files.charDef, files.unk,
            );

            const builder = new TokenizerBuilder();
            builder.setDictionaryInstance(dictionary);
            builder.setMode("normal");
            tokenizer = builder.build();

            document.getElementById('tokenize').disabled = false;
            document.getElementById('output').textContent = 'Ready!';
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

## Webpack の設定

Webpack 5 を使用する場合は、`asyncWebAssembly` experiment を有効にします：

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

次に、bundler ターゲットビルドを使用してインポートします：

```javascript
import { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-bundler';
import { loadDictionaryFiles } from 'lindera-wasm-bundler/opfs';

// OPFS から辞書を読み込み（セットアップは OPFS 辞書ストレージを参照）
const files = await loadDictionaryFiles("ipadic");
const dictionary = loadDictionaryFromBytes(
    files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
    files.dictWords, files.matrixMtx, files.charDef, files.unk,
);

const builder = new TokenizerBuilder();
builder.setDictionaryInstance(dictionary);
builder.setMode("normal");
const tokenizer = builder.build();
```

bundler ターゲットでは、`__wbg_init()` はバンドラーによって自動的に呼び出されます。

## Vite / Rollup のセットアップ

Vite は web ターゲットでの WASM をそのままサポートしています。ビルド済みの `pkg/` ディレクトリをプロジェクトに配置し、直接インポートします：

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from './pkg/lindera_wasm.js';
import { loadDictionaryFiles } from './pkg/opfs.js';

await __wbg_init();
// OPFS から辞書を読み込み、上記のように TokenizerBuilder を使用
```

bundler ターゲットで Vite を使用する場合は、[vite-plugin-wasm](https://github.com/nicolo-ribaudo/vite-plugin-wasm) プラグインが必要になることがあります：

```javascript
// vite.config.js
import wasm from 'vite-plugin-wasm';

export default {
    plugins: [wasm()],
};
```

## Chrome 拡張機能に関する注意事項

Manifest V3 を使用する Chrome 拡張機能では、デフォルトで `WebAssembly.compile` と `WebAssembly.instantiate` が制限されています。拡張機能で lindera-wasm を使用するには、Content Security Policy に `wasm-unsafe-eval` を追加する必要があります：

```json
{
    "content_security_policy": {
        "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'"
    }
}
```

`wasm-unsafe-eval` は WebAssembly の実行のみを許可し、任意の JavaScript `eval()` は許可しません。

## パフォーマンスのヒント

- **初期化は一度だけ**: `__wbg_init()` はアプリケーション起動時に一度だけ呼び出し、トークナイズリクエストごとには呼び出さないでください。
- **トークナイザーの再利用**: `Tokenizer` インスタンスは一度作成し、複数の `tokenize()` 呼び出しで再利用してください。
- **Web Workers**: 大量のトークナイズ処理を行う場合は、メインスレッドのブロックを避けるため、Web Worker での Lindera の実行を検討してください。

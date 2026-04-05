# lindera-wasm

Lindera の WebAssembly 版

![Screenshot from 2025-09-13 23-05-49](https://github.com/user-attachments/assets/a6ca165a-825c-4260-ba52-d76cd262a21f)

## デモアプリケーション

- <https://lindera.github.io/lindera/demo/>

## npm

- <https://www.npmjs.com/package/lindera-wasm-web> — Web 向け Lindera WASM
- <https://www.npmjs.com/package/lindera-wasm-bundler> — バンドラ向け Lindera WASM

## 辞書

ビルド済み辞書は [GitHub Releases](https://github.com/lindera/lindera/releases) から入手できます。
辞書アーカイブ（例: `lindera-ipadic-*.zip`）をダウンロードし、OPFS API を使用してランタイムで読み込みます。
辞書のダウンロードと読み込みの動作デモは[サンプルアプリケーション](example/)を参照してください。

## 使い方

### Web での使用

ブラウザ環境では `lindera-wasm-web` パッケージを使用します。
辞書はローカルパスまたは [GitHub Releases](https://github.com/lindera/lindera/releases) から OPFS API を使用してダウンロードし、ランタイムで読み込みます。

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

### バンドラでの使用（Webpack、Rollup など）

バンドラ環境では `lindera-wasm-bundler` パッケージを使用します。
辞書の読み込み方法は上記の Web での使用と同じです。

### トークンのプロパティ

各トークンオブジェクトには以下のプロパティがあります:

| プロパティ | 型 | 説明 |
| -------- | ---- | ----------- |
| `surface` | `string` | トークンの表層形 |
| `byteStart` | `number` | 元テキスト内の開始バイト位置 |
| `byteEnd` | `number` | 元テキスト内の終了バイト位置 |
| `position` | `number` | トークンの位置インデックス |
| `wordId` | `number` | 辞書内の単語 ID |
| `details` | `string[]` | 形態素詳細の配列 |

メソッド:

- `getDetail(index)`: 指定したインデックスの詳細を返します。見つからない場合は `undefined` を返します
- `toJSON()`: トークンをプレーンな JavaScript オブジェクトとして返します

### Vite プロジェクトの場合

`optimizeDeps` でこのパッケージを除外する必要があります:

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

### ブラウザ拡張機能の開発

vite.config.js で `cors` 設定を行います:

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

また、manifest.json の `content_security_policy` に `wasm-unsafe-eval` を含める必要があります:

```json
"content_security_policy": {
  "extension_pages": "script-src 'self' 'wasm-unsafe-eval';"
}
```

## 開発

### プロジェクト依存関係のインストール

- wasm-pack : <https://rustwasm.github.io/wasm-pack/installer/>

### リポジトリのセットアップ

```shell
# Clone the Lindera project repository
% git clone git@github.com:lindera/lindera.git
% cd lindera
```

### プロジェクトのビルド

```shell
% make wasm-build
```

### テストの実行

```shell
% make wasm-test
```

### サンプル Web アプリケーションのビルド

```shell
% cd example && npm install && npm run build && cp index.html dist/index.html
```

### サンプル Web アプリケーションの実行

```shell
% cd example && npm run start
```

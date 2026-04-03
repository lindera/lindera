# OPFS 辞書ストレージ

Lindera WASM は、Web ブラウザでの辞書の永続キャッシュのために OPFS（Origin Private File System）ヘルパーユーティリティを提供します。辞書を一度ダウンロードすれば、WASM バイナリに埋め込むことなく、セッションをまたいで再利用できます。

## 概要

OPFS ヘルパーは、WASM パッケージとともに別の JavaScript モジュール（`opfs.js`）として配布されます。ブラウザの Origin Private File System を使用して、辞書のダウンロード、保存、読み込み、管理を行う関数を提供します。

辞書は OPFS パス `lindera/dictionaries/<name>/` に保存されます。

## インポート

```javascript
import { downloadDictionary, loadDictionaryFiles, removeDictionary,
         listDictionaries, hasDictionary } from 'lindera-wasm-web/opfs';
```

## 関数

### `downloadDictionary(url, name, options?)`

辞書の zip アーカイブをダウンロードし、展開して、ファイルを OPFS に保存します。

アーカイブは、必要な 8 つの辞書ファイルを含む zip ファイルである必要があります。サブディレクトリにネストされていても構いません。

- **引数**:
  - `url` (string) -- 辞書 zip アーカイブの URL
  - `name` (string) -- 辞書の保存名（例: `"ipadic"`）
  - `options` (object, 省略可):
    - `onProgress` (function) -- 進捗コールバック
- **戻り値**: `Promise<void>`

```javascript
await downloadDictionary(
    "https://example.com/ipadic.zip",
    "ipadic",
    {
        onProgress: (progress) => {
            switch (progress.phase) {
                case "downloading":
                    console.log(`Downloading: ${progress.loaded}/${progress.total} bytes`);
                    break;
                case "extracting":
                    console.log("Extracting archive...");
                    break;
                case "storing":
                    console.log("Storing in OPFS...");
                    break;
                case "complete":
                    console.log("Done!");
                    break;
            }
        },
    },
);
```

#### 進捗コールバック

`onProgress` コールバックは以下の形式のオブジェクトを受け取ります:

| プロパティ | 型 | 説明 |
| --- | --- | --- |
| `phase` | `string` | `"downloading"`、`"extracting"`、`"storing"`、または `"complete"` |
| `loaded` | `number \| undefined` | ダウンロード済みバイト数（`"downloading"` フェーズのみ） |
| `total` | `number \| undefined` | 合計バイト数（判明している場合、`"downloading"` フェーズのみ） |

### `loadDictionaryFiles(name)`

OPFS から辞書ファイルを `Uint8Array` 値のオブジェクトとして読み込みます。

返されたオブジェクトは `loadDictionaryFromBytes()` にそのまま渡すことができます。

- **引数**: `name` (string) -- 辞書名（例: `"ipadic"`）
- **戻り値**: `Promise<DictionaryFiles>`

```javascript
const files = await loadDictionaryFiles("ipadic");
```

#### DictionaryFiles

| プロパティ | 型 | ソースファイル |
| --- | --- | --- |
| `metadata` | `Uint8Array` | `metadata.json` |
| `dictDa` | `Uint8Array` | `dict.da`（Double-Array Trie） |
| `dictVals` | `Uint8Array` | `dict.vals`（単語値データ） |
| `dictWordsIdx` | `Uint8Array` | `dict.wordsidx`（単語詳細インデックス） |
| `dictWords` | `Uint8Array` | `dict.words`（単語詳細） |
| `matrixMtx` | `Uint8Array` | `matrix.mtx`（連接コスト行列） |
| `charDef` | `Uint8Array` | `char_def.bin`（文字定義） |
| `unk` | `Uint8Array` | `unk.bin`（未知語辞書） |

### `removeDictionary(name)`

OPFS から辞書を削除します。

- **引数**: `name` (string) -- 削除する辞書名
- **戻り値**: `Promise<void>`

```javascript
await removeDictionary("ipadic");
```

### `listDictionaries()`

OPFS に保存されているすべての辞書を一覧表示します。

- **戻り値**: `Promise<string[]>` -- 辞書名の配列

```javascript
const names = await listDictionaries();
console.log(names); // 例: ["ipadic", "unidic"]
```

### `hasDictionary(name)`

OPFS に辞書が存在するかどうかを確認します。

- **引数**: `name` (string) -- 確認する辞書名
- **戻り値**: `Promise<boolean>`

```javascript
if (await hasDictionary("ipadic")) {
    console.log("Dictionary is cached");
}
```

## 完全なワークフロー

OPFS ベースの辞書を使用する一般的なワークフロー:

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles, hasDictionary } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    const DICT_NAME = "ipadic";
    const DICT_URL = "https://example.com/lindera-ipadic.zip";

    // キャッシュされていない場合は辞書をダウンロード
    if (!await hasDictionary(DICT_NAME)) {
        await downloadDictionary(DICT_URL, DICT_NAME, {
            onProgress: ({ phase, loaded, total }) => {
                if (phase === "downloading" && total) {
                    console.log(`${(loaded / total * 100).toFixed(1)}%`);
                }
            },
        });
    }

    // OPFS から辞書を読み込み
    const files = await loadDictionaryFiles(DICT_NAME);
    const dictionary = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
        files.dictWords, files.matrixMtx, files.charDef, files.unk,
    );

    // トークナイザーを構築
    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dictionary);
    builder.setMode("normal");
    const tokenizer = builder.build();

    // トークナイズ
    const tokens = tokenizer.tokenize("形態素解析を行います");
    tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
}

main();
```

## 必要な辞書ファイル

有効な辞書アーカイブには以下の 8 ファイルが含まれている必要があります:

| ファイル | 説明 |
| --- | --- |
| `metadata.json` | 辞書メタデータ（名前、エンコーディング、スキーマなど） |
| `dict.da` | Double-Array Trie 構造 |
| `dict.vals` | 単語値データ |
| `dict.wordsidx` | 単語詳細インデックス |
| `dict.words` | 単語詳細（形態素素性） |
| `matrix.mtx` | 連接コスト行列 |
| `char_def.bin` | 文字カテゴリ定義 |
| `unk.bin` | 未知語辞書 |

## ブラウザ互換性

OPFS は[安全なコンテキスト](https://developer.mozilla.org/ja/docs/Web/Security/Secure_Contexts)（HTTPS または localhost）が必要で、以下のブラウザでサポートされています:

- Chrome 86+
- Edge 86+
- Firefox 111+
- Safari 15.2+

zip の展開には `DecompressionStream` API を使用しており、以下が必要です:

- Chrome 80+
- Edge 80+
- Firefox 113+
- Safari 16.4+

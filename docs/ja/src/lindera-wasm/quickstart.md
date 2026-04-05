# クイックスタート

## Web（ブラウザ） -- OPFS 辞書ロード

推奨される方法は、OPFS ヘルパーを使用して辞書を実行時にダウンロードすることです：

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles, hasDictionary } from 'lindera-wasm-web/opfs';

async function main() {
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

    // トークナイザーを構築
    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dictionary);
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("関西国際空港限定トートバッグ");
    tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
}

main();
```

> **注意:** ビルド済み辞書を [GitHub Releases](https://github.com/lindera/lindera/releases) からダウンロードしてください。完全なワークフローは [OPFS 辞書ストレージ](./opfs.md) を参照してください。

期待される出力：

```text
関西国際空港    名詞,固有名詞,一般,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## 埋め込み辞書の使用（上級者向け）

`embed-*` feature フラグ付きでビルドした場合、埋め込み辞書を使用できます：

```javascript
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-web-ipadic';

async function main() {
    await __wbg_init();

    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
    builder.setMode("normal");
    const tokenizer = builder.build();

    const tokens = tokenizer.tokenize("関西国際空港限定トートバッグ");
    tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
}

main();
```

## フィルタの使用

トークナイズパイプラインに文字フィルタやトークンフィルタを追加できます：

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    // OPFS にキャッシュ済みの辞書を読み込み
    const files = await loadDictionaryFiles("ipadic");
    const dictionary = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
        files.dictWords, files.matrixMtx, files.charDef, files.unk,
    );

    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dictionary);
    builder.setMode("normal");

    // Add Unicode NFKC normalization
    builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });

    // Add a stop-tags filter to remove particles and auxiliary verbs
    builder.appendTokenFilter("japanese_stop_tags", {
        tags: ["助詞", "助動詞"]
    });

    const tokenizer = builder.build();
    const tokens = tokenizer.tokenize("Ｌｉｎｄｅｒａは形態素解析エンジンです");
    tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
}

main();
```

## N-Best トークナイズ

コスト順にランク付けされた複数のトークナイズ候補を取得します：

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
results.forEach((result, rank) => {
    console.log(`--- NBEST ${rank + 1} (cost=${result.cost}) ---`);
    result.tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
});
```

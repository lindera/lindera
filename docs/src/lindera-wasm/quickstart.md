# Quick Start

## Web (Browser) -- OPFS Dictionary Loading

The recommended approach is to download dictionaries at runtime using the OPFS helpers:

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { downloadDictionary, loadDictionaryFiles, hasDictionary } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    // Download dictionary if not cached
    if (!await hasDictionary("ipadic")) {
        await downloadDictionary(
            "https://github.com/lindera/lindera/releases/download/<version>/lindera-ipadic-<version>.zip",
            "ipadic",
        );
    }

    // Load dictionary from OPFS
    const files = await loadDictionaryFiles("ipadic");
    const dictionary = loadDictionaryFromBytes(
        files.metadata, files.dictDa, files.dictVals, files.dictWordsIdx,
        files.dictWords, files.matrixMtx, files.charDef, files.unk,
    );

    // Build tokenizer
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

> **Note:** Download a pre-built dictionary from [GitHub Releases](https://github.com/lindera/lindera/releases). See [OPFS Dictionary Storage](./opfs.md) for the full workflow.

Expected output:

```text
関西国際空港    名詞,固有名詞,一般,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## Using Embedded Dictionaries (Advanced)

If you built with an `embed-*` feature flag, you can use embedded dictionaries:

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

## Using Filters

You can add character filters and token filters to the tokenization pipeline:

```javascript
import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes } from 'lindera-wasm-web';
import { loadDictionaryFiles } from 'lindera-wasm-web/opfs';

async function main() {
    await __wbg_init();

    // Assume dictionary is already cached in OPFS
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

## N-Best Tokenization

Retrieve multiple tokenization candidates ranked by cost:

```javascript
const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);
results.forEach((result, rank) => {
    console.log(`--- NBEST ${rank + 1} (cost=${result.cost}) ---`);
    result.tokens.forEach(token => {
        console.log(`${token.surface}\t${token.details.join(',')}`);
    });
});
```

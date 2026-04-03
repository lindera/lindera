# Quick Start

## Web (Browser)

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

Expected output:

```text
関西国際空港    名詞,固有名詞,一般,*,*,*,関西国際空港,カンサイコクサイクウコウ,カンサイコクサイクーコー
限定    名詞,サ変接続,*,*,*,*,限定,ゲンテイ,ゲンテイ
トートバッグ    名詞,一般,*,*,*,*,*,*,*
```

## Using Filters

You can add character filters and token filters to the tokenization pipeline:

```javascript
import __wbg_init, { TokenizerBuilder } from 'lindera-wasm-web-ipadic';

async function main() {
    await __wbg_init();

    const builder = new TokenizerBuilder();
    builder.setDictionary("embedded://ipadic");
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

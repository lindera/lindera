# Text Processing Pipeline

Lindera Node.js supports a composable text processing pipeline that applies character filters before tokenization and token filters after tokenization. Filters are added to the `TokenizerBuilder` and executed in the order they are appended.

```text
Input Text
  --> Character Filters (preprocessing)
  --> Tokenization
  --> Token Filters (postprocessing)
  --> Output Tokens
```

> [!NOTE]
> This page shows a few commonly used filters as examples -- it is **not** the complete list.
> `lindera-analysis` ships 4 character filters and 18 token filters in total. See
> [Filters](../lindera-analysis/filters.md) for the full, authoritative catalogue of every
> character and token filter, including parameters and examples.

## Character Filters

Character filters transform the input text before tokenization.

### unicode_normalize

Applies Unicode normalization to the input text.

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
const tokenizer = builder.build();
```

Supported normalization forms: `"nfc"`, `"nfkc"`, `"nfd"`, `"nfkd"`.

### mapping

Replaces characters or strings according to a mapping table.

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendCharacterFilter("mapping", {
  mapping: {
    "\u30fc": "-",
    "\uff5e": "~",
  },
});
const tokenizer = builder.build();
```

### japanese_iteration_mark

Resolves Japanese iteration marks (odoriji) into their full forms.

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendCharacterFilter("japanese_iteration_mark", {
  normalize_kanji: true,
  normalize_kana: true,
});
const tokenizer = builder.build();
```

## Token Filters

Token filters transform or remove tokens after tokenization.

### lowercase

Converts token surface forms to lowercase.

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendTokenFilter("lowercase", {});
const tokenizer = builder.build();
```

### japanese_base_form

Replaces inflected forms with their base (dictionary) form using the morphological details from the dictionary.

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendTokenFilter("japanese_base_form", {});
const tokenizer = builder.build();
```

### japanese_stop_tags

Removes tokens whose part-of-speech matches any of the specified tags.

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendTokenFilter("japanese_stop_tags", {
  tags: ["助詞,格助詞,一般", "助詞,係助詞", "助詞,連体化", "助動詞"],
});
const tokenizer = builder.build();
```

> [!NOTE]
> Tags are normalized to exactly four comma-separated levels (missing levels are padded with `*`)
> and compared for exact equality against the first four part-of-speech details of each token.
> A bare `助詞` therefore never matches IPADIC particle tokens — they always carry a subcategory
> such as `助詞,係助詞` — while a bare `助動詞` does match, because auxiliary verbs have no
> subcategory (`助動詞,*,*,*`).

### japanese_keep_tags

Keeps only tokens whose part-of-speech matches one of the specified tags. All other tokens are removed. The following keeps only general nouns:

```javascript
const builder = new TokenizerBuilder();
builder.setDictionary("embedded://ipadic");
builder.appendTokenFilter("japanese_keep_tags", {
  tags: ["名詞,一般"],
});
const tokenizer = builder.build();
```

## Complete Pipeline Example

The following example combines multiple character filters and token filters into a single pipeline:

```javascript
const { TokenizerBuilder } = require("lindera");

const builder = new TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("embedded://ipadic");
// Preprocessing
builder.appendCharacterFilter("unicode_normalize", { kind: "nfkc" });
builder.appendCharacterFilter("japanese_iteration_mark", {
  normalize_kanji: true,
  normalize_kana: true,
});
// Postprocessing
builder.appendTokenFilter("japanese_base_form", {});
builder.appendTokenFilter("japanese_stop_tags", {
  tags: ["助詞,格助詞,一般", "助詞,係助詞", "助詞,連体化", "助動詞", "記号,句点", "記号,読点"],
});
builder.appendTokenFilter("lowercase", {});
const tokenizer = builder.build();

const tokens = tokenizer.tokenize("Ｌｉｎｄｅｒａは形態素解析を行うライブラリです。");
for (const token of tokens) {
  console.log(`${token.surface}\t${token.details.join(",")}`);
}
```

In this pipeline:

1. `unicode_normalize` converts full-width characters to half-width (NFKC normalization)
2. `japanese_iteration_mark` resolves iteration marks
3. `japanese_base_form` converts inflected tokens to base form
4. `japanese_stop_tags` removes particles, auxiliary verbs, and punctuation
5. `lowercase` normalizes alphabetic characters to lowercase

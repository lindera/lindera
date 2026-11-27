# Migrating from v7.0 to v7.1

This guide lists the breaking changes of v7.1.0. There are three, in the Rust API, and they are in a minor version by a decision of the maintainer ([#1440](https://github.com/lindera/lindera/issues/1440), [#1446](https://github.com/lindera/lindera/issues/1446), [#1466](https://github.com/lindera/lindera/issues/1466)): code that reads the tokens a tokenizer returns is not affected.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| **`Token` holds the result of a dependency analysis, and is `#[non_exhaustive]`** | Rust code that builds a `Token` with a struct expression, or that destructures all of its fields | Build the token with `Token::new`, and set the fields that differ afterwards |
| **`Dictionary::save_to_path` is removed** | Rust code that calls it, which panicked when it ran | Copy the directory of the dictionary |
| **`Metadata` has a new field, `skip_invalid_encoding`** | Rust code that builds a `Metadata` with a struct expression that names every field | Add `skip_invalid_encoding: false`, or end the expression with `..Default::default()` |

Code that reads the tokens a tokenizer returns is not affected: `surface`, `byte_start`, `details()`, `as_value()` and the other fields and methods are what they were.

v7.1 adds bunsetsu dependency analysis ([Lindera Dependency](./lindera-dependency.md)), and the change of `Token` comes with it. The rest of what v7.1 adds needs no change of existing code; [What does not change](#what-does-not-change) lists what a user may notice.

## `Token` is built with `Token::new`

`Token` gets a field for the result of a dependency analysis, read with `Token::dependency()`, which is `None` unless a token filter that analyzes dependencies ran ([#1438](https://github.com/lindera/lindera/issues/1438)). Adding a field breaks every struct expression of the type, so the type is `#[non_exhaustive]` from now on: a field that is added later will not break code again.

Outside the crate of the type, a `#[non_exhaustive]` struct cannot be built with a struct expression, and a pattern over its fields needs `..`. This is code that makes tokens by hand, which is mostly the tests of a custom token filter.

Before:

```rust
let token = Token {
    surface: Cow::Borrowed("東京都"),
    byte_start: 0,
    byte_end: 9,
    position: 0,
    position_length: 2,
    word_id: WordId::new(LexType::System, 0),
    dictionary: &dictionary,
    user_dictionary: None,
    details: Some(vec![Cow::Borrowed("名詞"), Cow::Borrowed("固有名詞")]),
};
```

After:

```rust
let token = Token::new(
    Cow::Borrowed("東京都"),
    0,
    9,
    0,
    WordId::new(LexType::System, 0),
    &dictionary,
    None,
)
.with_position_length(2)
.with_details(vec![Cow::Borrowed("名詞"), Cow::Borrowed("固有名詞")]);
```

- `Token::new` takes the surface, the byte offsets, the position, the word id, the dictionary and the user dictionary, in this order. It sets `position_length` to 1 and leaves the details and the dependency empty.
- `with_position_length` and `with_details` are new; they set the two fields a struct expression used to set. The fields are public as before, and can be assigned as well: `token.details = Some(...)`.
- A pattern that named every field needs `..` at its end: `let Token { surface, byte_start, .. } = token;`.

The compiler names each place: `cannot create non-exhaustive struct using struct expression` (E0639) for a struct expression, and ``..` required with struct marked as non-exhaustive`` (E0638) for a pattern.

## `Dictionary::save_to_path` is removed

`lindera_dictionary::dictionary::Dictionary::save_to_path` is gone ([#1446](https://github.com/lindera/lindera/issues/1446)). It was in the API since v1.0.0 and never had an implementation: it created the directory it was given and then panicked, in every call. Code that calls it no longer compiles:

```text
error[E0599]: no method named `save_to_path` found for struct `lindera_dictionary::dictionary::Dictionary` in the current scope
```

Nothing replaces it: Lindera does not write a loaded dictionary. A dictionary is a directory of files, and a copy of the directory is the same dictionary; [`lindera build`](./lindera-cli/commands.md) makes one from its sources, and [`lindera download`](./lindera-cli/commands.md) installs a published one.

## `Metadata` has a new field

`lindera_dictionary::dictionary::metadata::Metadata` has the field `skip_invalid_encoding: bool` ([#1466](https://github.com/lindera/lindera/issues/1466)), which the JUMAN dictionary needs: with it, the builder skips a row of a lexicon CSV file that is not text in the encoding of the dictionary, and warns of it (see [the `build` command](./lindera-cli/commands.md#rows-that-are-not-in-the-encoding)). `Metadata` is not `#[non_exhaustive]`, so a struct expression that names every field no longer compiles:

```text
error[E0063]: missing field `skip_invalid_encoding` in initializer of `lindera_dictionary::dictionary::metadata::Metadata`
```

Add `skip_invalid_encoding: false`, which is what every build did before, or end the expression with `..Default::default()`, which later fields then do not break. Code that reads a `metadata.json`, that starts from `Metadata::default()` or that calls `Metadata::new` is not affected: a `metadata.json` without the field reads as `false`, and one that has `false` is written without it.

## What does not change

- The JSON that `Token::as_value` and `lindera tokenize -o json` write has no new key for a token without a dependency.
- `Token::details` holds the fields of the dictionary, as before: the result of a dependency analysis is not written there.
- A token grows by the size of one pointer.
- The `lindera` command and the bindings have the `dependency` feature in their default builds: the token filter `japanese_dependency` is in the packages, and they depend on the `lindera-dependency` crate, which has no dependency of its own. The command grows by 134 kB. `default-features = false` gives the build without it. The libraries `lindera` and `lindera-analysis` have the feature only when it is named.
- No model is in a package or embedded by default; `lindera download dependency-ipadic` gets the one for IPADIC, and `dependency-unidic`, `dependency-unidic-cwj` and `dependency-unidic-csj` the ones for UniDic.
- There is a tenth dictionary, the [JUMAN dictionary](./lindera-jumandic.md) (`jumandic`, the crate `lindera-jumandic`, the feature `embed-jumandic`), built from mecab-jumandic 7.0-20130310 ([#1466](https://github.com/lindera/lindera/issues/1466)). No default build embeds it, and `lindera download jumandic` gets it. The dependency parser has no model for it.

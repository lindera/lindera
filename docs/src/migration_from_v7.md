# Migrating from v7.0

This guide lists the breaking changes made after v7.0.0 that are not released yet. Which version carries them, 7.1.0 or 8.0.0, is decided with the release ([#1440](https://github.com/lindera/lindera/issues/1440)); this page takes the name of that version then.

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| **`Token` holds the result of a dependency analysis, and is `#[non_exhaustive]`** | Rust code that builds a `Token` with a struct expression, or that destructures all of its fields | Build the token with `Token::new`, and set the fields that differ afterwards |

Code that reads the tokens a tokenizer returns is not affected: `surface`, `byte_start`, `details()`, `as_value()` and the other fields and methods are what they were.

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

## What does not change

- The JSON that `Token::as_value` and `lindera tokenize -o json` write has no new key for a token without a dependency.
- `Token::details` holds the fields of the dictionary, as before: the result of a dependency analysis is not written there.
- A token grows by the size of one pointer.

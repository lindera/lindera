# Migrating from v3 to v4

Lindera v4.0.0 is a major release that bundles the breaking changes that were
deliberately deferred during the v3 series. Each change is small on its own; this
guide lists every one so you can upgrade with confidence.

The breaking changes were verified mechanically against a `cargo public-api` diff of
the v3.0.7 and v4 public surfaces (committed under `public-api/` in the repository).

## Overview

| Change | Affects | What you do |
| --- | --- | --- |
| Default schema field names use `pos_detail_*` | Python, Node.js, Ruby, PHP | Update field names `middle_pos` / `small_pos` / `fine_pos` to `pos_detail_1` / `pos_detail_2` / `pos_detail_3` |
| `Token.details` is always a list | Python, Node.js, Ruby | Remove `null` / `None` / `nil` handling |
| Binding `Segmenter` removed | Python, WASM | Use the tokenizer instead |
| `LINDERA_CACHE` env var removed | Rust build, CLI | Use `LINDERA_DICTIONARIES_PATH` |
| User-dictionary binary format changed | All (prebuilt `.bin`) | Rebuild user dictionaries from their CSV source |
| `lindera-dictionary` viterbi internals encapsulated | Rust crate users | Use the new accessors |

The top-level `lindera` crate's public Rust API is unchanged between v3.0.7 and v4.

## Default dictionary schema field names

`Schema.create_default()` (and the default dictionary schema) now names the three
part-of-speech detail fields `pos_detail_1`, `pos_detail_2`, and `pos_detail_3`
(indices 5, 6, 7), instead of `middle_pos`, `small_pos`, and `fine_pos`. This makes
every binding match the core `lindera::dictionary::Schema::default()`, which already
used `pos_detail_*`.

This affects the Python, Node.js, Ruby, and PHP bindings. The WASM binding already
used `pos_detail_*`, so it is unchanged.

In Python:

```python
schema = Schema.create_default()
# v3: schema.fields[5] == "middle_pos"
# v4: schema.fields[5] == "pos_detail_1"

# v3
index = schema.get_field_index("middle_pos")
# v4
index = schema.get_field_index("pos_detail_1")
```

If you reference these field names by string anywhere (lookups, custom schemas,
serialized configuration), update them to the `pos_detail_*` form.

## `Token.details` is always a list

`Token.details` is now always a list of strings and is never `null` / `None` / `nil`.
A token with no details is represented by an empty list. Previously the Python,
Node.js, and Ruby bindings wrapped it in a nullable type even though it was always
populated in practice; the PHP and WASM bindings were already non-nullable.

In Python the type changes from `list[str] | None` to `list[str]`:

```python
# v3 — defensive null check was required by the type
if token.details is not None:
    pos = token.details[0]

# v4 — details is always a list
pos = token.details[0]
```

In Node.js the type changes from `Array<string> | null` to `Array<string>`, and in
Ruby from `Array | nil` to `Array`. Remove any `null` / `nil` checks accordingly.

## Binding `Segmenter` removed

The vestigial `Segmenter` wrappers were removed from the bindings. They had no
constructor and could not be used; segmentation has always been reachable through the
tokenizer.

- Python: the `lindera.segmenter` submodule and `lindera.segmenter.Segmenter` are gone.
- WASM: the `Segmenter` class export is gone.

Tokenize through the tokenizer instead:

```python
from lindera import Tokenizer, TokenizerBuilder

tokenizer = TokenizerBuilder().build()
tokens = tokenizer.tokenize("関西国際空港")
```

## `LINDERA_CACHE` environment variable removed

The deprecated `LINDERA_CACHE` build-time environment variable was removed. Use
`LINDERA_DICTIONARIES_PATH`, which has been the supported variable for several
releases:

```sh
# v3 (deprecated)
export LINDERA_CACHE=/path/to/dicts

# v4
export LINDERA_DICTIONARIES_PATH=/path/to/dicts
```

## User-dictionary binary format changed

User dictionaries now use the same 8-bit variant-count encoding as system
dictionaries (supporting up to 255 variants per surface, previously 31). As a result,
a user-dictionary `.bin` file built with v3 is decoded incorrectly by v4. There is no
format-version guard, so the failure is silent — tokens are produced, but with the
wrong details.

Rebuild user dictionaries from their CSV source with v4:

```sh
lindera build --user \
  --src user_dict.csv \
  --dest ./build \
  --metadata lindera-ipadic/metadata.json
```

If you load a user dictionary from a `.csv` file (rather than a prebuilt `.bin`), it is
rebuilt at load time and no action is needed.

## Rust library: `lindera-dictionary` viterbi internals

This affects only direct users of the `lindera-dictionary` crate; the `lindera` crate
API is unchanged.

The internal viterbi structs no longer expose public fields. Use the accessors
instead:

```rust
// v3 — direct field access
let id = word_id.id;
let cost = word_entry.word_cost;

// v4 — accessors
let id = word_id.id();
let cost = word_entry.word_cost();
```

Other changes in `lindera_dictionary::viterbi`:

- `EdgeType` was removed.
- `WordEntry` gained `new()`, `word_cost()`, and `word_id()`; `WordId` gained `id()`.
- `WordEntry::serialize`, `WordEntry::deserialize`, and `WordEntry::SERIALIZED_LEN` are
  no longer public.
- `util::read_aligned_file` and the `embedded_dictionary!` macro were added.

The complete machine-generated diff lives in `public-api/lindera-dictionary.diff` in
the repository.

## Upgrade checklist

- Replace `middle_pos` / `small_pos` / `fine_pos` with `pos_detail_1` / `pos_detail_2`
  / `pos_detail_3` (Python, Node.js, Ruby, PHP).
- Remove `null` / `None` / `nil` checks on `Token.details` (Python, Node.js, Ruby).
- Replace any use of the binding `Segmenter` with the tokenizer (Python, WASM).
- Replace `LINDERA_CACHE` with `LINDERA_DICTIONARIES_PATH`.
- Rebuild prebuilt user-dictionary `.bin` files from their CSV source.
- Switch direct `lindera-dictionary` viterbi field access to the new accessors (Rust).

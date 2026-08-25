# Lindera SudachiDict

Lindera SudachiDict is a Japanese dictionary crate based on [SudachiDict](https://github.com/WorksApplications/SudachiDict), the actively maintained dictionary used by the Sudachi morphological analyzer. It is updated several times a year upstream, so recent vocabulary (`令和`, `スマホ`, `推し活`, ...) is in-vocabulary. Each entry has 19 fields, including SudachiDict-specific columns such as the normalized form, split references, and synonym group IDs.

The dictionary source files are mirrored at [lindera/sudachidict](https://github.com/lindera/sudachidict). The bundled version is `20260723` (small + core + notcore lexicons).

> [!NOTE]
> This dictionary reproduces SudachiDict's vocabulary and lattice behavior, but not the Sudachi engine plugins (katakana/numeric joining, input normalization, A/B/C split modes). See [Behavioral differences from Sudachi](./sudachidict.md#behavioral-differences-from-sudachi) for details.

## Contents

- [Dictionary Format](./lindera-sudachidict/dictionary_format.md) -- Field definitions for system and user dictionaries
- [Build](./lindera-sudachidict/build.md) -- How to build the dictionary from source
- [Examples](./lindera-sudachidict/examples.md) -- Tokenization examples

## API Reference

- [lindera-sudachidict](https://docs.rs/lindera-sudachidict)

# Test data

Files in the lattice format of CaboCha (`cabocha -f1`) that `tests/lattice.rs`
reads and writes back, comparing the bytes. The sentences were written for
these tests; the feature columns follow IPADIC.

| File | What it holds |
| --- | --- |
| `dependency.lattice` | Bunsetsu lines with the positions and the score, comments before the sentences, coordination (`P`) |
| `chunks.lattice` | Bunsetsu lines with the head only, as the chunk layer of CaboCha writes them: each ends with a space |
| `morphemes.lattice` | Morphemes without bunsetsu lines, and an empty sentence |
| `surfaces.lattice` | Surfaces that look like other lines (`*`, `#`, `EOS`), a comma quoted and not quoted, named entity tags |

The surface and the features of a morpheme are separated by a tab, and the
lines of `chunks.lattice` end with a space: an editor that turns tabs into
spaces or strips the spaces at the end of a line breaks the files.

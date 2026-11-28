#!/usr/bin/env python3
"""How much the output of two dictionaries differs on the same sentences.

Reads the lattice files `lindera dependency convert` wrote for each
dictionary (convert_corpora.sh), and prints, for each pair of dictionaries
and each file, over the sentences both kept (matched by their text):

- the sentences with the same segmentation into morphemes,
- those with also the same part of speech (the first four feature columns)
  for every morpheme,
- those with the same bunsetsu, each with the same head.

A model is trained on the morphemes of one dictionary; the numbers say
whether it could serve another one.

Usage:
    compare_dictionaries.py <name>=<lattice directory> <name>=<lattice directory>...

    <name>               a label, such as unidic-cwj
    <lattice directory>  the directory with the lattice files of the
                         dictionary, such as data/unidic-cwj/lattice
"""

import itertools
import pathlib
import sys

FILES = ["ja_gsd-ud-train", "ja_gsd-ud-dev", "ja_gsd-ud-test", "ja_pud-ud-test"]


def sentences(path):
    """The sentences of a lattice file, by their text.

    Args:
        path: The lattice file.

    Returns:
        A dictionary from the text of a sentence to its morphemes (the
        surface and the part of speech of each) and its bunsetsu (the
        character each starts at, and its link).
    """
    result = {}
    morphemes, bunsetsu, offset = [], [], 0
    for line in path.read_text(encoding="utf-8").split("\n"):
        if line == "EOS":
            text = "".join(surface for surface, _ in morphemes)
            result[text] = (morphemes, bunsetsu)
            morphemes, bunsetsu, offset = [], [], 0
        elif line.startswith("* ") and "\t" not in line:
            bunsetsu.append((offset, line.split(" ")[2]))
        elif "\t" in line:
            surface, features = line.split("\t", 1)
            morphemes.append((surface, ",".join(features.split(",")[:4])))
            offset += len(surface)
    return result


def share(count, total):
    """A count as a share of a total, with both."""
    return f"{100 * count / total:.2f}% ({count}/{total})" if total else "-"


def main(arguments):
    if len(arguments) < 2 or any("=" not in one for one in arguments):
        sys.exit(__doc__)
    directories = {}
    for argument in arguments:
        name, directory = argument.split("=", 1)
        directories[name] = pathlib.Path(directory)
    for name in FILES:
        loaded = {
            label: sentences(directory / f"{name}.lattice")
            for label, directory in directories.items()
        }
        print(name, " ".join(f"{label}={len(loaded[label])}" for label in loaded))
        for first, second in itertools.combinations(loaded, 2):
            common = loaded[first].keys() & loaded[second].keys()
            same_morphemes = same_tags = same_bunsetsu = 0
            for text in common:
                a_morphemes, a_bunsetsu = loaded[first][text]
                b_morphemes, b_bunsetsu = loaded[second][text]
                if [s for s, _ in a_morphemes] == [s for s, _ in b_morphemes]:
                    same_morphemes += 1
                    if a_morphemes == b_morphemes:
                        same_tags += 1
                if a_bunsetsu == b_bunsetsu:
                    same_bunsetsu += 1
            total = len(common)
            print(
                f"  {first} / {second}: sentences in both {total};"
                f" same morphemes {share(same_morphemes, total)};"
                f" and same part of speech {share(same_tags, total)};"
                f" same bunsetsu and heads {share(same_bunsetsu, total)}"
            )


if __name__ == "__main__":
    main(sys.argv[1:])

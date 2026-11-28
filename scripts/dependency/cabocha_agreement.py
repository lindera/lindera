#!/usr/bin/env python3
"""Compares what CaboCha wrote with what `lindera dependency parse` wrote,
both in the lattice format. The numbers of cabocha_models.sh.

Usage:
    cabocha_agreement.py outputs <label> <output of CaboCha> <output of parse> [--differences <file>]
    cabocha_agreement.py text <label> <output of CaboCha> <output of parse> [--columns <count>]
    cabocha_agreement.py sentences <lattice> <text>

outputs    The two were given the same morphemes. One line with:

           - the sentences whose bunsetsu are the same, of all;
           - in those sentences, the bunsetsu with the same head, of the
             bunsetsu that have one (all but the last of a sentence), and
             the bunsetsu with the same positions of the head word and of
             the function word, of all;
           - the sentences that are the same in everything: the bunsetsu,
             the heads and the positions;
           - the largest difference between the two scores of a bunsetsu
             with the same head. A bunsetsu whose head is the last bunsetsu
             is left out: no decision is made for such an arc, CaboCha
             writes the score of the decision before, and `parse` writes 0.

           With `--differences`, the sentences that are not the same are
           written to the file, each with its number from 1, what differs
           first, and its two versions. It is an error when the two files
           do not hold the same sentences with the same morphemes.

text       The two were given the same text, and each tokenized it: CaboCha
           with MeCab and its dictionary, `parse` with Lindera. One line
           with:

           - the sentences with the same segmentation into morphemes;
           - the sentences where the morphemes also have the same first
             feature columns (the part of speech, the conjugation and the
             base form: what the features of a model are made from).
             `--columns` says how many: seven for IPADIC, the default,
             and five for the JUMAN dictionary;
           - of those, the sentences that are the same in everything.

sentences  Writes the sentences of a lattice file as text, one a line: the
           surfaces of each, joined. The input of `text`.
"""

import csv
import re
import sys

BUNSETSU = re.compile(r"^\* (\d+) (-?\d+)([A-Z])(?: (\d+)/(\d+))?(?: (-?[0-9.eE+-]+))?")
# The feature columns a schema of IPADIC reads: the default of `text`.
COLUMNS = 7


def sentences(path):
    """The sentences of a file: for each its bunsetsu, as (number of
    morphemes before it, head, positions, score), its morphemes, and its
    lines."""
    result = []
    bunsetsu, morphemes, lines = [], [], []
    with open(path, encoding="utf-8") as file:
        for line in file:
            line = line.rstrip("\n")
            if line == "EOS":
                result.append((bunsetsu, morphemes, lines))
                bunsetsu, morphemes, lines = [], [], []
                continue
            if not line:
                continue
            lines.append(line)
            match = BUNSETSU.match(line) if "\t" not in line else None
            if match:
                positions = (match.group(4), match.group(5))
                score = float(match.group(6)) if match.group(6) else None
                bunsetsu.append((len(morphemes), int(match.group(2)), positions, score))
            else:
                morphemes.append(line)
    if lines:
        sys.exit(f"{path}: the last sentence does not end with EOS")
    return result


def both(cabocha_path, parse_path):
    cabocha = sentences(cabocha_path)
    ours = sentences(parse_path)
    if len(cabocha) != len(ours):
        sys.exit(
            f"{cabocha_path} has {len(cabocha)} sentences, and {parse_path} {len(ours)}"
        )
    return cabocha, ours


def share(count, total):
    return f"{100 * count / total:.2f}% ({count}/{total})" if total else "- (0/0)"


def difference(theirs, mine):
    """What differs first between the analyses of a sentence, or `None`;
    with the counts of the bunsetsu and their largest difference of score."""
    counts = {
        "heads": 0,
        "same_heads": 0,
        "positions": 0,
        "same_positions": 0,
        "score": 0.0,
    }
    if [one[0] for one in theirs] != [one[0] for one in mine]:
        return "bunsetsu", counts
    what = None
    last = len(theirs) - 1
    for index, (one, other) in enumerate(zip(theirs, mine)):
        counts["positions"] += 1
        if one[2] == other[2]:
            counts["same_positions"] += 1
        else:
            what = what or f"positions of the bunsetsu {index}"
        if index == last:
            continue
        counts["heads"] += 1
        if one[1] == other[1]:
            counts["same_heads"] += 1
            decided = one[1] != last
            if decided and one[3] is not None and other[3] is not None:
                counts["score"] = max(counts["score"], abs(one[3] - other[3]))
        else:
            what = what or f"head of the bunsetsu {index}"
    return what, counts


def outputs(argv):
    differences = None
    if "--differences" in argv:
        index = argv.index("--differences")
        differences = argv[index + 1]
        argv = argv[:index] + argv[index + 2 :]
    if len(argv) != 3:
        sys.exit(__doc__)
    label, cabocha_path, parse_path = argv
    cabocha, ours = both(cabocha_path, parse_path)

    same_bunsetsu = same_all = 0
    heads = same_heads = positions = same_positions = 0
    largest = 0.0
    report = []
    for number, (theirs, mine) in enumerate(zip(cabocha, ours), start=1):
        if theirs[1] != mine[1]:
            sys.exit(f"sentence {number}: the morphemes of the two files differ")
        what, counts = difference(theirs[0], mine[0])
        if what != "bunsetsu":
            same_bunsetsu += 1
        heads += counts["heads"]
        same_heads += counts["same_heads"]
        positions += counts["positions"]
        same_positions += counts["same_positions"]
        largest = max(largest, counts["score"])
        if what is None:
            same_all += 1
        else:
            report.append((number, what, theirs[2], mine[2]))

    print(
        f"{label}: same bunsetsu {share(same_bunsetsu, len(ours))}"
        f" same head {share(same_heads, heads)}"
        f" same positions {share(same_positions, positions)}"
        f" same sentence {share(same_all, len(ours))}"
        f" largest score difference {largest:.6f}"
    )
    if differences is not None:
        with open(differences, "w", encoding="utf-8") as file:
            for number, what, theirs, mine in report:
                file.write(f"# sentence {number}: {what}\n# CaboCha\n")
                file.write("\n".join(theirs) + "\n# parse\n")
                file.write("\n".join(mine) + "\n\n")


def columns(morpheme, count):
    """The surface of a morpheme line and its first feature columns."""
    surface, features = morpheme.split("\t", 1)
    values = next(csv.reader([features]))
    values += ["*"] * (count - len(values))
    return surface, values[:count]


def text(argv):
    count = COLUMNS
    if "--columns" in argv:
        index = argv.index("--columns")
        count = int(argv[index + 1])
        argv = argv[:index] + argv[index + 2 :]
    if len(argv) != 3:
        sys.exit(__doc__)
    label, cabocha_path, parse_path = argv
    cabocha, ours = both(cabocha_path, parse_path)
    same_segmentation = same_morphemes = same_all = 0
    for theirs, mine in zip(cabocha, ours):
        one = [columns(morpheme, count) for morpheme in theirs[1]]
        other = [columns(morpheme, count) for morpheme in mine[1]]
        if [surface for surface, _ in one] != [surface for surface, _ in other]:
            continue
        same_segmentation += 1
        if one != other:
            continue
        same_morphemes += 1
        if difference(theirs[0], mine[0])[0] is None:
            same_all += 1
    print(
        f"{label}: same segmentation {share(same_segmentation, len(ours))}"
        f" same morphemes {share(same_morphemes, len(ours))}"
        f" of which the same sentence {share(same_all, same_morphemes)}"
    )


def write_sentences(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    lattice, output = argv
    with open(output, "w", encoding="utf-8") as file:
        file.writelines(
            "".join(morpheme.split("\t", 1)[0] for morpheme in morphemes) + "\n"
            for _, morphemes, _ in sentences(lattice)
        )


def main(argv):
    commands = {"outputs": outputs, "text": text, "sentences": write_sentences}
    if not argv or argv[0] not in commands:
        sys.exit(__doc__)
    commands[argv[0]](argv[1:])


if __name__ == "__main__":
    main(sys.argv[1:])

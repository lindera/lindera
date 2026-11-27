#!/usr/bin/env python3
"""Compares the morphemes MeCab gives for a text with those of `lindera
tokenize`, both in MeCab's output format. The numbers of mecab_morphemes.sh.

Usage:
    mecab_agreement.py <label> <output of MeCab> <output of lindera> <columns>... [--examples <count>]

    <columns>  A number of feature columns, such as 6: the sentences whose
               morphemes are the same in their surfaces and in that many
               columns are counted. Several numbers give several counts.

It prints:

- one line with the sentences that have the same segmentation, and for each
  `<columns>` the sentences that are also the same in those columns;
- where the segmentations differ, the first difference of each sentence
  counted by the kind of the characters MeCab has there;
- where only the features differ, the first column that differs, counted;
- with `--examples`, that many first differences of each kind, both sides.

A morpheme with fewer columns than are compared has `*` in the others, as
an unknown word of MeCab has for a dictionary whose `unk.def` is shorter
than its lexicon.

The two files have to hold the same number of sentences: it is an error
when they do not.
"""

import collections
import csv
import sys
import unicodedata


def sentences(path):
    """The sentences of a file: the morphemes of each, as (surface, columns)."""
    result, morphemes = [], []
    with open(path, encoding="utf-8") as file:
        for line in file:
            line = line.rstrip("\n")
            if line == "EOS":
                result.append(morphemes)
                morphemes = []
            elif "\t" in line:
                surface, features = line.split("\t", 1)
                morphemes.append((surface, next(csv.reader([features]))))
    if morphemes:
        sys.exit(f"{path}: the last sentence does not end with EOS")
    return result


def kind(text):
    """The kinds of the characters of a text, as `katakana` or `digit+latin`."""
    kinds = set()
    for character in text:
        name = unicodedata.name(character, "")
        if character.isdigit():
            kinds.add("digit")
        elif "LATIN" in name:
            kinds.add("latin")
        elif "KATAKANA" in name or character == "ー":
            kinds.add("katakana")
        elif "HIRAGANA" in name:
            kinds.add("hiragana")
        elif "CJK" in name:
            kinds.add("kanji")
        else:
            kinds.add("symbol")
    return "+".join(sorted(kinds)) or "nothing"


def first_difference(one, other):
    """The morphemes of the two sides from the first that differs to where
    the two meet again."""
    index = 0
    while index < min(len(one), len(other)) and one[index][0] == other[index][0]:
        index += 1
    left, right = index, index
    left_offset = len(one[left][0]) if left < len(one) else 0
    right_offset = len(other[right][0]) if right < len(other) else 0
    while left_offset != right_offset and (
        left + 1 < len(one) or right + 1 < len(other)
    ):
        if left_offset < right_offset and left + 1 < len(one):
            left += 1
            left_offset += len(one[left][0])
        elif right + 1 < len(other):
            right += 1
            right_offset += len(other[right][0])
        else:
            break
    return one[index : left + 1], other[index : right + 1]


def padded(columns, count):
    return (columns + ["*"] * count)[:count]


def shown(morphemes):
    return " / ".join(
        f"{surface} ({','.join(columns[:2])})" for surface, columns in morphemes
    )


def main(argv):
    examples = 0
    if "--examples" in argv:
        index = argv.index("--examples")
        examples = int(argv[index + 1])
        argv = argv[:index] + argv[index + 2 :]
    if len(argv) < 4:
        sys.exit(__doc__)
    label, mecab_path, lindera_path = argv[:3]
    counts = [int(count) for count in argv[3:]]
    mecab = sentences(mecab_path)
    lindera = sentences(lindera_path)
    if len(mecab) != len(lindera):
        sys.exit(
            f"{mecab_path} has {len(mecab)} sentences, and {lindera_path} {len(lindera)}"
        )

    same_segmentation = 0
    same_columns = dict.fromkeys(counts, 0)
    kinds = collections.Counter()
    columns = collections.Counter()
    samples = collections.defaultdict(list)
    for theirs, ours in zip(mecab, lindera):
        if [surface for surface, _ in theirs] != [surface for surface, _ in ours]:
            one, other = first_difference(theirs, ours)
            what = kind("".join(surface for surface, _ in one))
            kinds[what] += 1
            if len(samples[what]) < examples:
                samples[what].append((shown(one), shown(other)))
            continue
        same_segmentation += 1
        widest = max(counts)
        differs = None
        for (surface, one), (_, other) in zip(theirs, ours):
            one, other = padded(one, widest), padded(other, widest)
            for column in range(widest):
                if one[column] != other[column]:
                    if differs is None or column < differs:
                        differs = column
                        sample = (surface, one[column], other[column])
                    break
        for count in counts:
            if differs is None or differs >= count:
                same_columns[count] += 1
        if differs is not None:
            columns[differs + 1] += 1
            what = f"column {differs + 1}"
            if len(samples[what]) < examples:
                samples[what].append(
                    (f"{sample[0]}: {sample[1]}", f"{sample[0]}: {sample[2]}")
                )

    def share(count):
        return f"{100 * count / len(mecab):.2f}% ({count}/{len(mecab)})"

    line = f"{label}: same segmentation {share(same_segmentation)}"
    for count in counts:
        line += f" same in {count} columns {share(same_columns[count])}"
    print(line)
    if kinds:
        print(
            f"{label}: the first difference of a segmentation, by its characters: "
            + ", ".join(f"{what} {count}" for what, count in kinds.most_common())
        )
    if columns:
        print(
            f"{label}: the first column that differs, with the same segmentation: "
            + ", ".join(
                f"{column} in {count}" for column, count in sorted(columns.items())
            )
        )
    for what, pairs in samples.items():
        for one, other in pairs:
            print(f"  [{what}] mecab: {one}")
            print(f"  [{what}] lindera: {other}")


if __name__ == "__main__":
    main(sys.argv[1:])

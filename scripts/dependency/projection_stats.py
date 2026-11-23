#!/usr/bin/env python3
"""Lays the bunsetsu of UD Japanese over the morphemes of a dictionary and counts.

An implementation in Python of the projection of `lindera-dependency
convert` (lindera-dependency/src/corpus/project.rs), written independently
of it. The sentences that conversion_stats.py keeps are tokenized with the
`lindera` command; the bunsetsu boundaries are laid over the morphemes by
character offsets, and where a boundary falls inside a morpheme the bunsetsu
are merged up to the next boundary that fits, as tools/kc2cabocha.pl of
CaboCha does.

Usage:
    python3 -I -B projection_stats.py <lindera command> <dictionary> <output directory> <CoNLL-U file>...

    <lindera command>  the `lindera` binary, such as target/release/lindera
                       built with `--features embed-ipadic`
    <dictionary>       what its `-d` takes, such as embedded://ipadic

For each file it writes <output directory>/<name>.lattice, in the lattice
format with bunsetsu lines that hold the head only, and prints the counts.
compare_outputs.py compares these files with the ones `convert` writes.
"""

import os
import subprocess
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from conversion_stats import read_conllu, restore_coordination, to_bunsetsu, violations


def strip_white_space(text):
    return "".join(text.split())


def tokenize(command, dictionary, texts):
    """Tokenizes one text per line; returns the (surface, features) of each."""
    output = subprocess.run(
        [command, "tokenize", "-d", dictionary, "-o", "mecab"],
        input="\n".join(texts) + "\n",
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    sentences, current = [], []
    for line in output.split("\n"):
        if line == "EOS":
            sentences.append(current)
            current = []
        elif line:
            surface, features = line.split("\t", 1)
            current.append((surface, features))
    assert len(sentences) == len(texts), (len(sentences), len(texts))
    return sentences


def project(bunsetsu_lengths, heads, types, morpheme_lengths):
    """Lays bunsetsu over morphemes, both given by their lengths in characters.

    Returns (groups, heads, types, merged, fit, boundaries): the number of
    morphemes of each new bunsetsu, its head and arc type, the bunsetsu lost
    by merging, and the boundaries that are morpheme boundaries among all.
    """
    bunsetsu_ends, position = [], 0
    for length in bunsetsu_lengths:
        position += length
        bunsetsu_ends.append(position)
    morpheme_ends, position = [], 0
    for length in morpheme_lengths:
        position += length
        morpheme_ends.append(position)
    morpheme_end_set = set(morpheme_ends)
    boundaries = len(bunsetsu_ends) - 1
    fit = sum(1 for end in bunsetsu_ends[:-1] if end in morpheme_end_set)
    # A new bunsetsu ends where a bunsetsu of the corpus ends on a morpheme
    # boundary; the last bunsetsu of the corpus in it gives its head.
    sources = [b for b, end in enumerate(bunsetsu_ends) if end in morpheme_end_set]
    groups, start, index = [], 0, 0
    for b in sources:
        while morpheme_ends[index] != bunsetsu_ends[b]:
            index += 1
        index += 1
        groups.append(index - start)
        start = index
    new_heads, new_types = [], []
    for b in sources:
        head = heads[b]
        target = -1
        if head != -1:
            for j, source in enumerate(sources):
                if source >= head:
                    target = j
                    break
        new_heads.append(target)
        new_types.append(types[b])
    merged = len(bunsetsu_ends) - len(sources)
    return groups, new_heads, new_types, merged, fit, boundaries


def main(argv):
    command, dictionary, out_dir = argv[0], argv[1], argv[2]
    os.makedirs(out_dir, exist_ok=True)
    total = Counter()
    for path in argv[3:]:
        kept = []
        for comments, rows in read_conllu(path):
            result = to_bunsetsu(rows)
            if result is None:
                continue
            starts, heads, deprels, _ = result
            new_heads, types = restore_coordination(heads, deprels)
            if violations(new_heads):
                continue
            text = next(
                c.split("=", 1)[1].strip() for c in comments if c.startswith("# text =")
            )
            ends = starts[1:] + [len(rows)]
            forms = [
                "".join(strip_white_space(row[1]) for row in rows[start:end])
                for start, end in zip(starts, ends)
            ]
            kept.append((text, forms, new_heads, types))
        tokens = tokenize(command, dictionary, [sentence[0] for sentence in kept])
        stats = Counter()
        name = os.path.basename(path).replace(".conllu", ".lattice")
        with open(os.path.join(out_dir, name), "w", encoding="utf-8") as out:
            for (text, forms, heads, types), morphemes in zip(kept, tokens):
                stats["sentences in"] += 1
                if "".join(strip_white_space(m[0]) for m in morphemes) != "".join(
                    forms
                ):
                    stats["dropped, text that does not match"] += 1
                    continue
                groups, new_heads, new_types, merged, fit, boundaries = project(
                    [len(form) for form in forms],
                    heads,
                    types,
                    [len(strip_white_space(m[0])) for m in morphemes],
                )
                stats["boundaries"] += boundaries
                stats["boundaries on a morpheme boundary"] += fit
                stats["bunsetsu before"] += len(forms)
                stats["bunsetsu merged"] += merged
                if merged > 0:
                    stats["sentences with merged bunsetsu"] += 1
                if violations(new_heads):
                    stats["dropped, no tree after merging"] += 1
                    continue
                stats["sentences out"] += 1
                stats["bunsetsu out"] += len(groups)
                stats["morphemes out"] += len(morphemes)
                index = 0
                for i, size in enumerate(groups):
                    out.write(f"* {i} {new_heads[i]}{new_types[i]} \n")
                    out.writelines(
                        f"{surface}\t{features}\n"
                        for surface, features in morphemes[index : index + size]
                    )
                    index += size
                out.write("EOS\n")
        print(name)
        for key in sorted(stats):
            print(f"  {key}: {stats[key]}")
        total.update(stats)
    print("total")
    for key in sorted(total):
        print(f"  {key}: {total[key]}")
    fit = total["boundaries on a morpheme boundary"]
    print(f"  boundaries that fit %: {100 * fit / total['boundaries']:.2f}")
    print(
        f"  bunsetsu merged %: {100 * total['bunsetsu merged'] / total['bunsetsu before']:.2f}"
    )


if __name__ == "__main__":
    main(sys.argv[1:])

#!/usr/bin/env python3
"""The baseline "every bunsetsu depends on the next one", from an answer file.

Counts, on files in the lattice format with bunsetsu, how many heads are
right when every bunsetsu but the last depends on the next one, by the three
levels of CaboCha's eval.cpp (level 0 counts every bunsetsu, level 1 leaves
out the last of each sentence, level 2 the last two), and the sentences with
every head right. Written independently of `lindera dependency evaluate`,
to check its numbers.

Usage:
    python3 -I -B baseline_next.py [--write <directory>] <lattice file>...

With --write, it also writes each file with the heads replaced by the next
bunsetsu, as an input for `lindera dependency evaluate --system`.
"""

import os
import sys


def read(path):
    """The sentences of a file as (heads, lines)."""
    sentences, heads, lines = [], [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            lines.append(line)
            if line == "EOS":
                sentences.append((heads, lines))
                heads, lines = [], []
            elif line.startswith("* ") and "\t" not in line:
                heads.append(int(line.split(" ")[2].rstrip("DPAI")))
    return sentences


def main(argv):
    out_dir = None
    if argv and argv[0] == "--write":
        out_dir = argv[1]
        argv = argv[2:]
        os.makedirs(out_dir, exist_ok=True)
    for path in argv:
        correct = [0, 0, 0]
        total = [0, 0, 0]
        sentence_correct = sentence_total = 0
        # The lines of the file with the heads replaced, written at the end.
        written = []
        for heads, lines in read(path):
            count = len(heads)
            if count == 0:
                continue
            guess = [i + 1 if i + 1 < count else -1 for i in range(count)]
            for level in range(3):
                for i in range(count - level):
                    total[level] += 1
                    correct[level] += heads[i] == guess[i]
            sentence_total += 1
            sentence_correct += heads == guess
            index = 0
            for line in lines:
                if line.startswith("* ") and "\t" not in line:
                    written.append(f"* {index} {guess[index]}D \n")
                    index += 1
                else:
                    written.append(line + "\n")
        if out_dir:
            name = os.path.join(out_dir, os.path.basename(path))
            with open(name, "w", encoding="utf-8") as out:
                out.writelines(written)
        print(os.path.basename(path))
        for level in range(3):
            share = 100 * correct[level] / total[level]
            print(f"  level {level}: {share:.2f}% ({correct[level]}/{total[level]})")
        share = 100 * sentence_correct / sentence_total
        print(f"  sentences: {share:.2f}% ({sentence_correct}/{sentence_total})")


if __name__ == "__main__":
    main(sys.argv[1:])

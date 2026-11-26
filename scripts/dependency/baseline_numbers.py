#!/usr/bin/env python3
"""Prints the numbers of a baseline from what `lindera-dependency evaluate
--json` wrote. The lines of baselines.sh.

Usage:
    baseline_numbers.py numbers <label> <given.json> <end.json>
    baseline_numbers.py differ <label> <evaluation.json>
    baseline_numbers.py same-bunsetsu <evaluation.json>

numbers        One line with the three numbers of a tool on a file:
               `<given.json>` is the evaluation of its output with the
               bunsetsu of the answer, `<end.json>` of its output from the
               morphemes.
differ         The arcs on which two outputs differ: `<evaluation.json>` is
               the evaluation of one against the other, with the same
               bunsetsu.
same-bunsetsu  Fails unless the two outputs of an evaluation have the same
               bunsetsu.
"""

import json
import sys


def load(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


def numbers(label, given_path, end_path):
    level1 = load(given_path)["dependency"]["level1"]
    end = load(end_path)
    arcs = end["end_to_end"]["arcs"]
    chunking = end["chunking"]
    print(
        f"{label}:"
        f" level 1 {100 * level1['correct'] / level1['total']:.2f}"
        f" ({level1['correct']}/{level1['total']})"
        f" end to end {100 * arcs['correct'] / arcs['total']:.2f}"
        f" ({arcs['correct']}/{arcs['total']})"
        f" chunking F {100 * chunking['f_score']:.2f}"
        f" (P {chunking['correct']}/{chunking['system']},"
        f" R {chunking['correct']}/{chunking['gold']})"
    )


def differ(label, path):
    level1 = load(path)["dependency"]["level1"]
    print(
        f"{label}: the two differ on {level1['total'] - level1['correct']}"
        f" of {level1['total']} arcs"
    )


def same_bunsetsu(path):
    chunking = load(path)["chunking"]
    if not chunking["correct"] == chunking["gold"] == chunking["system"]:
        sys.exit(f"{path}: the bunsetsu of the two outputs differ")


def main(argv):
    commands = {
        "numbers": (numbers, 3),
        "differ": (differ, 2),
        "same-bunsetsu": (same_bunsetsu, 1),
    }
    if not argv or argv[0] not in commands or len(argv) - 1 != commands[argv[0]][1]:
        sys.exit(__doc__)
    commands[argv[0]][0](*argv[1:])


if __name__ == "__main__":
    main(sys.argv[1:])

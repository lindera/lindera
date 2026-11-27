#!/usr/bin/env python3
"""Counts the morpheme boundaries two chunkers decide differently, against
the answer. What #1464 compared the chunker of J.DepP and the chunker of
lindera-dependency with.

Usage:
    chunker_differences.py <answer> <first> <second> [--kinds <count>] [--examples <count>]

The three files are in the lattice format and hold the same sentences with
the same morphemes: the answer, and the outputs of two chunkers. A boundary
is the place before a morpheme that is not the first of its sentence; it is
a break when a bunsetsu starts there.

It prints:

- the boundaries, the breaks of the answer, and for each chunker its wrong
  boundaries: the breaks it misses and the breaks it adds;
- the boundaries where the two differ, by which of the two is right and by
  whether the answer has a break there, and the p-value of McNemar's exact
  test on the boundaries only one of the two has right;
- with `--kinds`, that many of the kinds of those boundaries, the largest
  first: the parts of speech before and after (two levels of each), with
  how often each chunker is right;
- with `--examples`, that many boundaries of each of the eight largest
  kinds, with three morphemes on each side: `||` where the answer has a
  break, and `--` where it has none.
"""

import collections
import math
import sys


def sentences(path):
    """The sentences of a lattice file: for each its morphemes as
    (surface, features), and the indexes where a bunsetsu starts."""
    result = []
    morphemes, starts = [], set()
    with open(path, encoding="utf-8") as file:
        for line in file:
            line = line.rstrip("\n")
            if line == "EOS":
                result.append((morphemes, starts))
                morphemes, starts = [], set()
            elif line.startswith("* ") and "\t" not in line:
                starts.add(len(morphemes))
            elif line:
                surface, _, features = line.partition("\t")
                morphemes.append((surface, features.split(",")))
    return result


def kind(features):
    """Two levels of the part of speech of a morpheme."""
    return "-".join(level for level in features[:2] if level != "*")


def mcnemar(first, second):
    """The two-sided p-value of McNemar's exact test."""
    total = first + second
    if total == 0:
        return 1.0
    larger = max(first, second)
    tail = sum(math.comb(total, count) for count in range(larger, total + 1))
    return min(1.0, 2 * tail / 2**total)


def option(arguments, name):
    """Takes `name <count>` out of the arguments, and gives the count."""
    if name not in arguments:
        return 0
    index = arguments.index(name)
    count = int(arguments[index + 1])
    del arguments[index : index + 2]
    return count


def main():
    arguments = sys.argv[1:]
    kinds_shown = option(arguments, "--kinds")
    examples = option(arguments, "--examples")
    if len(arguments) != 3:
        sys.exit(__doc__)
    answer, first, second = (sentences(path) for path in arguments)
    if not len(answer) == len(first) == len(second):
        sys.exit("the files do not hold the same number of sentences")

    boundaries = breaks = 0
    wrong = {"first": collections.Counter(), "second": collections.Counter()}
    differ = collections.Counter()
    kinds = collections.defaultdict(collections.Counter)
    samples = collections.defaultdict(list)
    for (morphemes, gold), (one, ones), (other, others) in zip(answer, first, second):
        surfaces = [surface for surface, _ in morphemes]
        if surfaces != [surface for surface, _ in one] or surfaces != [
            surface for surface, _ in other
        ]:
            sys.exit("the files do not hold the same morphemes")
        for index in range(1, len(morphemes)):
            boundaries += 1
            is_break = index in gold
            breaks += is_break
            decided = {"first": index in ones, "second": index in others}
            for name, value in decided.items():
                if value != is_break:
                    wrong[name]["missed" if is_break else "added"] += 1
            if decided["first"] == decided["second"]:
                continue
            right = "first" if decided["first"] == is_break else "second"
            differ[(right, "a break" if is_break else "no break")] += 1
            key = f"{kind(morphemes[index - 1][1])} | {kind(morphemes[index][1])}"
            kinds[key][right] += 1
            context = (
                " ".join(surfaces[max(0, index - 3) : index])
                + (" || " if is_break else " -- ")
                + " ".join(surfaces[index : index + 3])
            )
            samples[key].append(f"the {right} is right: {context}")

    print(f"boundaries {boundaries}, breaks in the answer {breaks}")
    for name in ("first", "second"):
        counts = wrong[name]
        print(
            f"the {name}: wrong {sum(counts.values())} (missed {counts['missed']}, added {counts['added']})"
        )
    only = {
        name: sum(count for (right, _), count in differ.items() if right == name)
        for name in ("first", "second")
    }
    print(
        f"the two differ on {sum(differ.values())} boundaries:"
        f" only the first right {only['first']}, only the second right {only['second']},"
        f" p {mcnemar(only['first'], only['second']):.4f}"
    )
    for (right, what), count in sorted(differ.items()):
        print(f"  the {right} is right, the answer has {what}: {count}")
    ordered = sorted(kinds.items(), key=lambda item: (-sum(item[1].values()), item[0]))
    if kinds_shown:
        print(
            f"{len(ordered)} kinds of boundary, by the parts of speech before | after: first right, second right"
        )
        for key, counts in ordered[:kinds_shown]:
            print(
                f"  {sum(counts.values()):3d}  {key}: {counts['first']}, {counts['second']}"
            )
    if examples:
        for key, _ in ordered[:8]:
            print(f"--- {key}")
            for sample in samples[key][:examples]:
                print(f"  {sample}")


if __name__ == "__main__":
    main()

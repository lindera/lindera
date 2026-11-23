#!/usr/bin/env python3
"""Converts the word dependencies of UD Japanese to bunsetsu dependencies and counts.

An implementation in Python of the steps `lindera-dependency convert` takes
before it tokenizes (lindera-dependency/src/corpus/convert.rs), written
independently of it, to check the numbers:

1. A word with `BunsetuBILabel=B` starts a bunsetsu.
2. In each bunsetsu, the word whose head is outside it gives the head of the
   bunsetsu.
3. The bunsetsu with the word whose head is the root is the root.
4. Coordination is put back in the form of the Kyoto corpus: for a bunsetsu
   `a` with the conjuncts `b` and `c` and the head `h`, the arcs become
   `a -> b`, `b -> c`, `c -> h`, with the type `P` between conjuncts.
5. A sentence that does not have one root that is its last bunsetsu, every
   head to the right and no crossing arcs is dropped.

Usage:
    python3 -I -B conversion_stats.py <CoNLL-U file>...

For each file it prints the sentences, the bunsetsu, the bunsetsu with
exactly one word whose head is outside, the sentences kept with and without
step 4, the arcs of type `P`, and the causes of the sentences dropped.
"""

import sys
from collections import Counter


def read_conllu(path):
    """Yields (comments, rows) for each sentence; a row is the 10 columns."""
    comments, rows = [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                if rows:
                    yield comments, rows
                comments, rows = [], []
            elif line.startswith("#"):
                comments.append(line)
            else:
                rows.append(line.split("\t"))
    if rows:
        yield comments, rows


def misc(row):
    """The MISC column as a dictionary."""
    return dict(pair.split("=", 1) for pair in row[9].split("|") if "=" in pair)


def to_bunsetsu(rows):
    """Steps 1 to 3.

    Returns (starts, heads, deprels, outside): the first word of each
    bunsetsu, its head (-1 for the root), the relation of the word that
    gives it, and the number of words whose head is outside the bunsetsu;
    None when the first word starts no bunsetsu.
    """
    starts = [i for i, row in enumerate(rows) if misc(row).get("BunsetuBILabel") == "B"]
    if not starts or starts[0] != 0:
        return None
    bunsetsu_of = []
    current = -1
    for i in range(len(rows)):
        if current + 1 < len(starts) and starts[current + 1] == i:
            current += 1
        bunsetsu_of.append(current)
    heads = [None] * len(starts)
    deprels = [None] * len(starts)
    outside = [0] * len(starts)
    for i, row in enumerate(rows):
        head = int(row[6])
        own = bunsetsu_of[i]
        head_bunsetsu = -1 if head == 0 else bunsetsu_of[head - 1]
        if head_bunsetsu != own:
            outside[own] += 1
            heads[own] = head_bunsetsu
            deprels[own] = row[7]
    return starts, heads, deprels, outside


def restore_coordination(heads, deprels):
    """Step 4. Returns (heads, arc types)."""
    count = len(heads)
    conjuncts = [[] for _ in range(count)]
    for b in range(count):
        if deprels[b] == "conj" and heads[b] is not None and heads[b] >= 0:
            conjuncts[heads[b]].append(b)
    new_heads = list(heads)
    types = ["D"] * count

    def assign(bunsetsu, target, arc_type, seen):
        if bunsetsu in seen:
            return
        seen.add(bunsetsu)
        chain = conjuncts[bunsetsu]
        if not chain:
            new_heads[bunsetsu] = target
            types[bunsetsu] = arc_type
            return
        new_heads[bunsetsu] = chain[0]
        types[bunsetsu] = "P"
        for i, conjunct in enumerate(chain):
            if i + 1 < len(chain):
                assign(conjunct, chain[i + 1], "P", seen)
            else:
                assign(conjunct, target, arc_type, seen)

    seen = set()
    for b in range(count):
        if deprels[b] != "conj":
            assign(b, heads[b], "D", seen)
    return new_heads, types


def violations(heads):
    """Step 5: the ways the heads are not the tree the parsers assume."""
    count = len(heads)
    found = set()
    roots = [b for b in range(count) if heads[b] == -1]
    if len(roots) != 1 or roots[0] != count - 1:
        found.add("root")
    for b in range(count):
        if heads[b] is not None and 0 <= heads[b] <= b:
            found.add("left")
    for a in range(count):
        head = heads[a]
        if head is None or head <= a:
            continue
        for c in range(a + 1, head):
            inner = heads[c]
            if inner is not None and inner > head:
                found.add("crossing")
    return found


def main(paths):
    for path in paths:
        stats = Counter()
        causes = Counter()
        for _, rows in read_conllu(path):
            stats["sentences"] += 1
            result = to_bunsetsu(rows)
            if result is None:
                stats["first word starts no bunsetsu"] += 1
                continue
            starts, heads, deprels, outside = result
            stats["bunsetsu"] += len(starts)
            stats["bunsetsu with one word that leaves"] += sum(
                1 for o in outside if o == 1
            )
            stats["arcs to the left"] += sum(
                1 for b, h in enumerate(heads) if h is not None and 0 <= h < b
            )
            stats["arcs to the left that are conj"] += sum(
                1
                for b, h in enumerate(heads)
                if h is not None and 0 <= h < b and deprels[b] == "conj"
            )
            if not violations(heads):
                stats["kept without step 4"] += 1
            new_heads, types = restore_coordination(heads, deprels)
            found = violations(new_heads)
            if found:
                for cause in found:
                    causes[cause] += 1
                stats["dropped"] += 1
                continue
            stats["kept"] += 1
            stats["kept bunsetsu"] += len(starts)
            stats["kept arcs"] += len(starts) - 1
            stats["kept arcs of type P"] += sum(
                1 for b, t in enumerate(types) if t == "P" and new_heads[b] != -1
            )
        sentences = stats["sentences"]
        print(path.rsplit("/", 1)[-1])
        for key in sorted(stats):
            print(f"  {key}: {stats[key]}")
        print(
            f"  kept without step 4 %: {100 * stats['kept without step 4'] / sentences:.2f}"
        )
        print(f"  kept %: {100 * stats['kept'] / sentences:.2f}")
        print(
            f"  P % of arcs: {100 * stats['kept arcs of type P'] / max(stats['kept arcs'], 1):.2f}"
        )
        print(f"  causes: {dict(sorted(causes.items()))}")


if __name__ == "__main__":
    main(sys.argv[1:])

#!/usr/bin/env python3
"""Converts between the lattice format and what J.DepP reads and writes.

J.DepP reads the lattice format almost as it is, with two differences this
script takes care of:

- It takes a line by its first character: `*` is a bunsetsu line and `#` a
  comment. A morpheme whose surface starts with one of them (`*`, `#1`) is
  read as a bunsetsu line, which stops J.DepP, or dropped as a comment. Such
  a surface is given to J.DepP with the full-width character instead (`＊`,
  `＃`), and the morphemes of the output are taken from the file J.DepP was
  given, not from what it wrote.
- What it writes with `-v -1` is not the lattice format: the labels of the
  chunker are columns of the morpheme lines, and the head the parser found
  is a second field of the bunsetsu line.

Usage:
    jdepp_format.py input <lattice> <output> [--morphemes]
    jdepp_format.py chunked <output of jdepp -I 1 -v -1> <lattice> <output>
    jdepp_format.py parsed <output of jdepp -I 2 -v -1, or -I 0> <lattice> <output>

input    The file for J.DepP: the lattice file with the surfaces escaped.
         With `--morphemes`, without its bunsetsu lines: the input of
         `jdepp -I 0`.
chunked  The bunsetsu of J.DepP's chunker in the lattice format, each
         depending on the next, as `lindera dependency chunk` writes them.
parsed   The bunsetsu and the heads J.DepP wrote, in the lattice format.

`<lattice>` is the file `input` was made from: its morphemes are written,
and the output of J.DepP has to have the same number of sentences and of
morphemes in each.
"""

import re
import sys

BUNSETSU = re.compile(r"^\* (\d+) (-?\d+)([A-Z])(?: |$)")
ESCAPES = {"*": "＊", "#": "＃"}


def sentences(path):
    """The sentences of a file: the lines of each, without `EOS`."""
    result, lines = [], []
    with open(path, encoding="utf-8") as file:
        for line in file:
            line = line.rstrip("\n")
            if line == "EOS":
                result.append(lines)
                lines = []
            elif line:
                lines.append(line)
    if lines:
        sys.exit(f"{path}: the last sentence does not end with EOS")
    return result


def is_bunsetsu(line):
    return BUNSETSU.match(line) is not None


def escaped(line):
    """A morpheme line as J.DepP is given it."""
    return ESCAPES.get(line[0], line[0]) + line[1:]


def morphemes_of(lines):
    return [line for line in lines if not is_bunsetsu(line)]


def write_input(argv):
    lattice, output = argv[0], argv[1]
    only_morphemes = "--morphemes" in argv[2:]
    with open(output, "w", encoding="utf-8") as file:
        for lines in sentences(lattice):
            for line in lines:
                if is_bunsetsu(line):
                    if not only_morphemes:
                        file.write(line.rstrip(" ") + "\n")
                else:
                    file.write(escaped(line) + "\n")
            file.write("EOS\n")


def aligned(system_path, lattice_path):
    """The sentences of an output of J.DepP with those of the lattice file."""
    system = [
        [line for line in lines if not line.startswith("#")]
        for lines in sentences(system_path)
    ]
    gold = sentences(lattice_path)
    if len(system) != len(gold):
        sys.exit(
            f"{system_path} has {len(system)} sentences, and {lattice_path} {len(gold)}"
        )
    for number, (ours, theirs) in enumerate(zip(gold, system), start=1):
        expected = morphemes_of(ours)
        found = [line for line in theirs if not line.startswith("* ")]
        if len(expected) != len(found):
            sys.exit(
                f"{system_path}: sentence {number} has {len(found)} morphemes, "
                f"and {lattice_path} {len(expected)}"
            )
        for ours_line, theirs_line in zip(expected, found):
            surface = escaped(ours_line).split("\t", 1)[0]
            if theirs_line.split("\t", 1)[0] != surface:
                sys.exit(
                    f"{system_path}: sentence {number}: the morpheme "
                    f"{theirs_line.split(chr(9), 1)[0]!r} is not {surface!r}"
                )
        yield expected, theirs


def write_sentence(file, bunsetsu):
    """Writes a sentence: for each bunsetsu its head, the type of its arc
    and its morphemes."""
    for index, (head, arc, lines) in enumerate(bunsetsu):
        file.write(f"* {index} {head}{arc} \n")
        for line in lines:
            file.write(line + "\n")
    file.write("EOS\n")


def write_chunked(argv):
    system, lattice, output = argv
    with open(output, "w", encoding="utf-8") as file:
        for expected, theirs in aligned(system, lattice):
            bunsetsu = []
            tokens = [line for line in theirs if not line.startswith("* ")]
            for ours_line, theirs_line in zip(expected, tokens):
                # `surface<TAB>features<TAB>B B@0.99`: the label of the
                # answer, then the one of the chunker.
                label = theirs_line.rsplit("\t", 1)[1].split(" ")[1][0]
                if label == "B" or not bunsetsu:
                    bunsetsu.append([0, "D", []])
                bunsetsu[-1][2].append(ours_line)
            for index, one in enumerate(bunsetsu):
                one[0] = index + 1 if index + 1 < len(bunsetsu) else -1
            write_sentence(file, bunsetsu)


def write_parsed(argv):
    system, lattice, output = argv
    with open(output, "w", encoding="utf-8") as file:
        for number, (expected, theirs) in enumerate(aligned(system, lattice), start=1):
            bunsetsu = []
            remaining = iter(expected)
            for line in theirs:
                if line.startswith("* "):
                    fields = line.split(" ")
                    # `* 1 2D` from the morphemes; `* 1 2D 3D@0.83` with
                    # the bunsetsu of the answer: the head of the answer,
                    # then the one of the parser.
                    head = fields[3] if len(fields) > 3 else fields[2]
                    match = re.match(r"^(-?\d+)([A-Z])", head)
                    if match is None:
                        sys.exit(f"{system}: sentence {number}: no head in {line!r}")
                    bunsetsu.append([int(match.group(1)), match.group(2), []])
                else:
                    if not bunsetsu:
                        sys.exit(
                            f"{system}: sentence {number}: a morpheme before a bunsetsu"
                        )
                    bunsetsu[-1][2].append(next(remaining))
            write_sentence(file, bunsetsu)


def main(argv):
    commands = {"input": write_input, "chunked": write_chunked, "parsed": write_parsed}
    if len(argv) < 3 or argv[0] not in commands:
        sys.exit(__doc__)
    commands[argv[0]](argv[1:])


if __name__ == "__main__":
    main(sys.argv[1:])

#!/usr/bin/env python3
"""Compares the lattice files of projection_stats.py with those of `convert`.

The two are written independently, in Python and in Rust, so equal files
check both. They are compared line by line. `convert` quotes a feature
column that holds a comma, as CSV does, and the `lindera` command the Python
script reads does not: a line that differs only by that quoting is counted
apart and accepted.

Usage:
    python3 -I -B compare_outputs.py <directory of projection_stats.py> <directory of convert> <name>...

    <name>  a file name without its extension, such as ja_gsd-ud-train

It prints one line per file and ends with COMPARE_OK or COMPARE_FAILED.
"""

import csv
import sys


def unquote(line):
    """Reads the features of a morpheme line as CSV and joins them unquoted."""
    if "\t" not in line:
        return line
    surface, features = line.split("\t", 1)
    columns = next(csv.reader([features]))
    return surface + "\t" + ",".join(columns)


def main(argv):
    python_dir, rust_dir = argv[0], argv[1]
    failed = False
    for name in argv[2:]:
        with open(f"{python_dir}/{name}.lattice", encoding="utf-8") as f:
            python_lines = f.read().split("\n")
        with open(f"{rust_dir}/{name}.lattice", encoding="utf-8") as f:
            rust_lines = f.read().split("\n")
        identical = quoted = different = 0
        if len(python_lines) != len(rust_lines):
            print(
                f"{name}: the numbers of lines differ:"
                f" python={len(python_lines)} rust={len(rust_lines)}"
            )
            failed = True
            continue
        for number, (python_line, rust_line) in enumerate(
            zip(python_lines, rust_lines), 1
        ):
            if python_line == rust_line:
                identical += 1
            elif unquote(rust_line) == python_line:
                quoted += 1
            else:
                different += 1
                if different <= 5:
                    print(f"{name}:{number}: python={python_line!r} rust={rust_line!r}")
        status = "OK" if different == 0 else "DIFFERENT"
        print(
            f"{name}: {status} lines={len(rust_lines)} identical={identical}"
            f" quoting_only={quoted} different={different}"
        )
        failed = failed or different > 0
    print("COMPARE_FAILED" if failed else "COMPARE_OK")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

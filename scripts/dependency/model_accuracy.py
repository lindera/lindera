"""Measures a model file on the files it was not trained on, and compares
the counts with the ones the documentation gives.

Converts the dev and test files of UD Japanese-GSD and the test file of UD
Japanese-PUD with the dictionary of the model, parses each with the model,
from the
bunsetsu of the answer and from the morphemes, and counts with
`lindera dependency evaluate`: the dependency accuracy with the bunsetsu of
the answer (level 1), the dependency accuracy from the morphemes (end to
end), and the F of the chunking.

    python3 -I -B model_accuracy.py <command> <model> <corpora directory> <work directory> [--dict <URI or path>] [--expect <file>]

`<command>` is `lindera` built with `embed-ipadic` (the default build has
`train` and `dependency`),
`<corpora directory>` holds `gsd/` and `pud/` (fetch_corpora.sh), and the
lattice files go to `<work directory>`. `--dict` is the dictionary the
files are tokenized with, which has to be the one of the model: the
embedded IPADIC when it is left out, and the directory of the dictionary
for a model for UniDic. With `--expect`, the counts have to be those of the
file, `model_accuracy_<dictionary>.json` for the model of build_model.sh,
or the script fails: `release.yml` runs it on the model it is about to
publish, so that a release does not carry a model whose numbers are not the
ones of the documentation.
"""

import json
import os
import subprocess
import sys

FILES = [
    ("ja_gsd-ud-dev", "gsd"),
    ("ja_gsd-ud-test", "gsd"),
    ("ja_pud-ud-test", "pud"),
]


def run(args, cwd=None):
    return subprocess.run(args, check=True, capture_output=True, text=True, cwd=cwd)


def parsed(command, layer, model, gold, output):
    """Parses a lattice file from an input layer, and evaluates the output."""
    run(
        [
            command,
            "dependency",
            "parse",
            "--input-layer",
            layer,
            "--model",
            model,
            "--input",
            gold,
            "--output",
            output,
        ]
    )
    result = run(
        [
            command,
            "dependency",
            "evaluate",
            "--gold",
            gold,
            "--system",
            output,
            "--json",
        ]
    )
    return json.loads(result.stdout)


def measure(command, model, corpora, work, dictionary):
    """The counts of the model for each file."""
    counts = {}
    for name, corpus in FILES:
        gold = os.path.join(work, f"{name}.lattice")
        # From the directory of the corpus, as convert_corpora.sh does.
        run(
            [
                command,
                "dependency",
                "convert",
                "--dict",
                dictionary,
                "--input",
                f"{name}.conllu",
                "--output",
                gold,
            ],
            cwd=os.path.join(corpora, corpus),
        )
        given = parsed(
            command, "bunsetsu", model, gold, os.path.join(work, f"{name}.gold.lattice")
        )
        end = parsed(
            command, "morphemes", model, gold, os.path.join(work, f"{name}.end.lattice")
        )
        level1 = given["dependency"]["level1"]
        arcs = end["end_to_end"]["arcs"]
        counts[name] = {
            "level 1": [level1["correct"], level1["total"]],
            "end to end": [arcs["correct"], arcs["total"]],
            "chunking F": f"{100 * end['chunking']['f_score']:.2f}",
        }
    return counts


def main(argv):
    expect = None
    if "--expect" in argv:
        at = argv.index("--expect")
        expect = argv[at + 1]
        argv = argv[:at] + argv[at + 2 :]
    dictionary = "embedded://ipadic"
    if "--dict" in argv:
        at = argv.index("--dict")
        dictionary = argv[at + 1]
        argv = argv[:at] + argv[at + 2 :]
        # A directory is given to a command that runs in another one.
        if "://" not in dictionary:
            dictionary = os.path.abspath(dictionary)
    command, model, corpora, work = (os.path.abspath(one) for one in argv)
    os.makedirs(work, exist_ok=True)
    counts = measure(command, model, corpora, work, dictionary)
    for name, one in counts.items():
        level1, end = one["level 1"], one["end to end"]
        print(
            f"{name}: level 1 {100 * level1[0] / level1[1]:.2f} ({level1[0]}/{level1[1]})"
            f" end to end {100 * end[0] / end[1]:.2f} ({end[0]}/{end[1]})"
            f" chunking F {one['chunking F']}"
        )
    if expect is None:
        return 0
    with open(expect, encoding="utf-8") as file:
        expected = json.load(file)
    if counts == expected:
        print(f"the counts are those of {os.path.basename(expect)}")
        return 0
    for name in sorted(set(counts) | set(expected)):
        if counts.get(name) != expected.get(name):
            print(
                f"{name}: measured {counts.get(name)}, expected {expected.get(name)}",
                file=sys.stderr,
            )
    print(f"the counts are NOT those of {expect}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

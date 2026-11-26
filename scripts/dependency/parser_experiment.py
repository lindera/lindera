#!/usr/bin/env python3
"""Trains and evaluates the parser of lindera-dependency for one setting.

The driver of parser_experiments.sh. Every number comes from the
`lindera-dependency` command (`train-parser`, `train-chunker`, `parse`,
`evaluate`).

Usage:
    python3 -I -B parser_experiment.py cv <command> <folds directory> <work directory> <name> [train-parser options...]
    python3 -I -B parser_experiment.py full <command> <lattice directory> <work directory> <name> <files> [train-parser options...]
    python3 -I -B parser_experiment.py compare <command> <work directory> <first name> <second name>

cv       Trains on fold<i>.train.lattice and parses fold<i>.test.lattice on
         its own bunsetsu, for the 10 folds `lindera-dependency split`
         writes, the folds at the same time. The outputs and the answers
         are joined in the order of the folds under
         <work directory>/cv/<name>/ and evaluated as one file.
full     Trains the chunker with its defaults and the parser on
         <lattice directory>/ja_gsd-ud-train.lattice, into one model file,
         and evaluates on <files> (names without extension, separated by
         commas): on the bunsetsu of the answer, and from the morphemes.
compare  Compares the outputs of two settings of `cv` on the same answer:
         the pairs of arcs, and the p-value of McNemar's exact test.

Each call prints one line per result. The size of a model and the time of a
training are read from what `train-parser` writes to the standard error.
"""

import json
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

FOLDS = 10


def run(args):
    return subprocess.run(args, check=True, capture_output=True, text=True)


def train(command, data, model, options):
    """Trains a parser; returns the lines `key: value` of the summary."""
    result = run(
        [command, "train-parser", "--input", data, "--output", model] + options
    )
    summary = {}
    for line in result.stderr.split("\n"):
        if ": " in line:
            key, value = line.rsplit(": ", 1)
            summary[key] = value
    return summary


def evaluate(command, gold, system):
    result = run([command, "evaluate", "--gold", gold, "--system", system, "--json"])
    return json.loads(result.stdout)


def count(part):
    """A count of an evaluation, as `percent (correct/total)`."""
    return f"{100 * part['ratio']:.2f} ({part['correct']}/{part['total']})"


def parse(command, layer, model, source, output):
    """The command line that parses a lattice file from an input layer."""
    return [
        command,
        "parse",
        "--input-layer",
        layer,
        "--model",
        model,
        "--input",
        source,
        "--output",
        output,
    ]


def fold(command, folds, out, options, index):
    """Trains and parses one fold; returns the summary of the training."""
    model = os.path.join(out, f"fold{index}.model")
    summary = train(
        command, os.path.join(folds, f"fold{index}.train.lattice"), model, options
    )
    test = os.path.join(folds, f"fold{index}.test.lattice")
    output = os.path.join(out, f"fold{index}.out.lattice")
    run(parse(command, "bunsetsu", model, test, output))
    os.remove(model)
    return summary


def cv(argv):
    command, folds, work, name, options = argv[0], argv[1], argv[2], argv[3], argv[4:]
    out = os.path.join(work, "cv", name)
    os.makedirs(out, exist_ok=True)
    with ThreadPoolExecutor(max_workers=FOLDS) as pool:
        summaries = list(
            pool.map(
                lambda index: fold(command, folds, out, options, index), range(FOLDS)
            )
        )
    system_path = os.path.join(out, "system.lattice")
    gold_path = os.path.join(out, "gold.lattice")
    with (
        open(system_path, "w", encoding="utf-8") as system,
        open(gold_path, "w", encoding="utf-8") as gold,
    ):
        for index in range(FOLDS):
            output = os.path.join(out, f"fold{index}.out.lattice")
            with open(output, encoding="utf-8") as f:
                system.write(f.read())
            with open(
                os.path.join(folds, f"fold{index}.test.lattice"), encoding="utf-8"
            ) as f:
                gold.write(f.read())
            os.remove(output)
    dependency = evaluate(command, gold_path, system_path)["dependency"]

    def mean(key):
        return sum(float(summary[key]) for summary in summaries) / FOLDS

    passes = sum(int(summary["passes"].split()[0]) for summary in summaries) / FOLDS
    stopped = sum("not converged" in summary["passes"] for summary in summaries)
    print(
        f"cv {name}: level 1 {count(dependency['level1'])}"
        f" sentences {count(dependency['sentences'])}"
        f" per fold: seconds {mean('seconds'):.1f} passes {passes:.0f}"
        f"{f' ({stopped} not converged)' if stopped else ''}"
        f" features {mean('feature strings'):.0f} pair weights {mean('pair weights'):.0f}"
        f" bytes {mean('bytes'):.0f}",
        flush=True,
    )


def full(argv):
    command, data, work, name = argv[0], argv[1], argv[2], argv[3]
    files, options = argv[4].split(","), argv[5:]
    out = os.path.join(work, "full", name)
    os.makedirs(out, exist_ok=True)
    training = os.path.join(data, "ja_gsd-ud-train.lattice")
    chunker = os.path.join(out, "chunker.model")
    # The chunker is for the dictionary the parser is trained for.
    named = []
    if "--dictionary-name" in options:
        named = options[options.index("--dictionary-name") :][:2]
    run([command, "train-chunker", "--input", training, "--output", chunker] + named)
    model = os.path.join(out, "model")
    summary = train(command, training, model, ["--chunker", chunker] + options)
    print(
        f"full {name}: passes {summary['passes']} updates {summary['updates']}"
        f" features {summary['feature strings']} pair weights {summary['pair weights']}"
        f" bytes {summary['bytes']} seconds {summary['seconds']}",
        flush=True,
    )
    for file in files:
        gold = os.path.join(data, f"{file}.lattice")
        output = os.path.join(out, f"{file}.gold.lattice")
        run(parse(command, "bunsetsu", model, gold, output))
        given = evaluate(command, gold, output)
        output = os.path.join(out, f"{file}.end.lattice")
        run(parse(command, "morphemes", model, gold, output))
        end = evaluate(command, gold, output)
        print(
            f"full {name} {file}: level 1 {count(given['dependency']['level1'])}"
            f" end to end {count(end['end_to_end']['arcs'])}"
            f" chunking F {100 * end['chunking']['f_score']:.2f}",
            flush=True,
        )


def compare(argv):
    command, work, first, second = argv
    gold = os.path.join(work, "cv", first, "gold.lattice")
    result = run(
        [
            command,
            "evaluate",
            "--gold",
            gold,
            "--system",
            os.path.join(work, "cv", first, "system.lattice"),
            "--compare",
            os.path.join(work, "cv", second, "system.lattice"),
            "--json",
        ]
    )
    comparison = json.loads(result.stdout)
    first_level = 100 * comparison["first"]["dependency"]["level1"]["ratio"]
    second_level = 100 * comparison["second"]["dependency"]["level1"]["ratio"]
    pairs = comparison["paired_arcs"]
    print(
        f"compare {first} ({first_level:.2f}) and {second} ({second_level:.2f}):"
        f" arcs both {pairs['both']} first only {pairs['first_only']}"
        f" second only {pairs['second_only']} neither {pairs['neither']}"
        f" p {pairs['p_value']:.4f}",
        flush=True,
    )


if __name__ == "__main__":
    {"cv": cv, "full": full, "compare": compare}[sys.argv[1]](sys.argv[2:])

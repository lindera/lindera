#!/usr/bin/env python3
"""Trains and evaluates the chunker of lindera-dependency for one setting.

The driver of chunker_experiments.sh. Every number comes from the
`lindera dependency` command (`train-chunker`, `chunk`, `evaluate`).

Usage:
    python3 -I -B chunker_experiment.py cv <command> <folds directory> <work directory> <name> [train-chunker options...]
    python3 -I -B chunker_experiment.py full <command> <lattice directory> <work directory> <name> <files> [train-chunker options...]
    python3 -I -B chunker_experiment.py compare <command> <work directory> <first name> <second name>

cv       Trains on fold<i>.train.lattice and segments fold<i>.test.lattice
         again, for the 10 folds `lindera dependency split` writes. The
         outputs and the answers are joined in the order of the folds under
         <work directory>/cv/<name>/ and evaluated as one file.
full     Trains on <lattice directory>/ja_gsd-ud-train.lattice and evaluates
         on <files>: names without extension, separated by commas.
compare  Compares the outputs of two settings of `cv` on the same answer:
         the pairs of sentences by their segmentation, and the p-value of
         McNemar's exact test.

Each call prints one line. The size of a model is read from what
`train-chunker` writes to the standard error.
"""

import json
import os
import subprocess
import sys

FOLDS = 10


def run(args):
    return subprocess.run(args, check=True, capture_output=True, text=True)


def train(command, data, model, options):
    """Trains a model; returns the lines `key: value` of the summary."""
    result = run(
        [command, "dependency", "train-chunker", "--input", data, "--output", model]
        + options
    )
    summary = {}
    for line in result.stderr.split("\n"):
        if ": " in line:
            key, value = line.rsplit(": ", 1)
            summary[key] = value
    return summary


def evaluate(command, gold, system):
    result = run(
        [
            command,
            "dependency",
            "evaluate",
            "--gold",
            gold,
            "--system",
            system,
            "--json",
        ]
    )
    return json.loads(result.stdout)


def chunking(evaluation):
    """The chunking accuracy of an evaluation, as one line."""
    c = evaluation["chunking"]
    sentences = c["sentences"]
    return (
        f"F {100 * c['f_score']:.2f}"
        f" P {100 * c['precision']:.2f} ({c['correct']}/{c['system']})"
        f" R {100 * c['recall']:.2f} ({c['correct']}/{c['gold']})"
        f" sentences {sentences['correct']}/{sentences['total']}"
    )


def cv(argv):
    command, folds, work, name, options = argv[0], argv[1], argv[2], argv[3], argv[4:]
    out = os.path.join(work, "cv", name)
    os.makedirs(out, exist_ok=True)
    features = weights = 0
    system_path = os.path.join(out, "system.lattice")
    gold_path = os.path.join(out, "gold.lattice")
    with (
        open(system_path, "w", encoding="utf-8") as system,
        open(gold_path, "w", encoding="utf-8") as gold,
    ):
        for fold in range(FOLDS):
            model = os.path.join(out, f"fold{fold}.model")
            summary = train(
                command,
                os.path.join(folds, f"fold{fold}.train.lattice"),
                model,
                options,
            )
            features += int(summary["feature strings"])
            weights += int(summary["weights"])
            test = os.path.join(folds, f"fold{fold}.test.lattice")
            output = os.path.join(out, f"fold{fold}.out.lattice")
            run(
                [
                    command,
                    "dependency",
                    "chunk",
                    "--model",
                    model,
                    "--input",
                    test,
                    "--output",
                    output,
                ]
            )
            with open(output, encoding="utf-8") as f:
                system.write(f.read())
            with open(test, encoding="utf-8") as f:
                gold.write(f.read())
            os.remove(model)
            os.remove(output)
    evaluation = evaluate(command, gold_path, system_path)
    print(
        f"cv {name}: {chunking(evaluation)}"
        f" features/fold {features // FOLDS} weights/fold {weights // FOLDS}",
        flush=True,
    )


def full(argv):
    command, data, work, name = argv[0], argv[1], argv[2], argv[3]
    files, options = argv[4].split(","), argv[5:]
    out = os.path.join(work, "full", name)
    os.makedirs(out, exist_ok=True)
    model = os.path.join(out, "chunker.model")
    summary = train(
        command, os.path.join(data, "ja_gsd-ud-train.lattice"), model, options
    )
    print(
        f"full {name}: iterations {summary['iterations']}"
        f" features {summary['feature strings']}"
        f" weights {summary['weights']} bytes {summary['bytes']}",
        flush=True,
    )
    for file in files:
        gold = os.path.join(data, f"{file}.lattice")
        output = os.path.join(out, f"{file}.lattice")
        run(
            [
                command,
                "dependency",
                "chunk",
                "--model",
                model,
                "--input",
                gold,
                "--output",
                output,
            ]
        )
        print(
            f"full {name} {file}: {chunking(evaluate(command, gold, output))}",
            flush=True,
        )


def compare(argv):
    command, work, first, second = argv
    gold = os.path.join(work, "cv", first, "gold.lattice")
    result = run(
        [
            command,
            "dependency",
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
    first_f = 100 * comparison["first"]["chunking"]["f_score"]
    second_f = 100 * comparison["second"]["chunking"]["f_score"]
    pairs = comparison["paired_segmentations"]
    print(
        f"compare {first} ({first_f:.2f}) and {second} ({second_f:.2f}):"
        f" segmentations both {pairs['both']} first only {pairs['first_only']}"
        f" second only {pairs['second_only']} neither {pairs['neither']}"
        f" p {pairs['p_value']:.4f}",
        flush=True,
    )


if __name__ == "__main__":
    {"cv": cv, "full": full, "compare": compare}[sys.argv[1]](sys.argv[2:])

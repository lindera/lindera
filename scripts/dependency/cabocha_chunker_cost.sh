#!/usr/bin/env bash
# The cost of CaboCha's chunker (`cabocha-learn -e chunk -c`), which the
# baseline of CaboCha takes at 4: the F of the chunking for each candidate,
# on dev and test with a model trained on the whole train file, and over a
# 10-fold cross-validation of the train file.
#
# Usage: cabocha_chunker_cost.sh [--tools <tools directory>] [<data directory>] [<cost>...]
#   --tools  Where build_baselines.sh installed the tools. Defaults to
#            <data directory>/tools.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The models and the outputs go to
#   <data directory>/baselines/chunker-cost.
#   <cost>   The candidates; 0.5 1 2 4 8 16 when none is given.
#
# One line per cost. A training takes about 4 seconds, so the 11 trainings
# of a cost take under a minute.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
tools=
data=
costs=()
while [ $# -gt 0 ]; do
    case "$1" in
    --tools)
        tools=$2
        shift 2
        ;;
    -*)
        echo "unknown option $1" >&2
        exit 2
        ;;
    *)
        if [ -z "$data" ]; then data=$1; else costs+=("$1"); fi
        shift
        ;;
    esac
done
data=$(readlink -f "${data:-$here/data}")
tools=$(readlink -f "${tools:-$data/tools}")
if [ ${#costs[@]} -eq 0 ]; then
    costs=(0.5 1 2 4 8 16)
fi
lattice=$data/lattice
work=$data/baselines/chunker-cost
libexec=$tools/libexec/cabocha
mkdir -p "$work"

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features train
evaluate="$root/target/release/lindera-dependency"
if [ ! -d "$work/folds" ]; then
    "$evaluate" split --input "$lattice/ja_gsd-ud-train.lattice" --folds 10 \
        --output-dir "$work/folds" > /dev/null
fi

# chunked <model> <gold> <output>: the bunsetsu of the chunker for the
# morphemes of a file, in the lattice format.
chunked() {
    grep -v -E '^\* [0-9]+ -?[0-9]+[A-Z] *$' "$2" > "$3.morphemes"
    "$tools/bin/cabocha" -I 1 -O 2 -f 1 -n 0 -P IPA -t UTF-8 -M "$1" "$3.morphemes" > "$3"
}

# f_score <gold> <output>: the F of the chunking, with its counts.
f_score() {
    "$evaluate" evaluate --gold "$1" --system "$2" --json |
        python3 -I -B -c 'import json, sys
c = json.load(sys.stdin)["chunking"]
print("%.2f (P %d/%d, R %d/%d)" % (100 * c["f_score"], c["correct"], c["system"], c["correct"], c["gold"]))'
}

for cost in "${costs[@]}"; do
    out=$work/c$cost
    rm -rf "${out:?}"
    mkdir -p "$out"
    "$libexec/cabocha-learn" -e chunk -P IPA -t UTF-8 -c "$cost" \
        "$lattice/ja_gsd-ud-train.lattice" "$out/chunk.model" > "$out/learn.log" 2>&1
    chunked "$out/chunk.model" "$lattice/ja_gsd-ud-dev.lattice" "$out/dev.lattice"
    chunked "$out/chunk.model" "$lattice/ja_gsd-ud-test.lattice" "$out/test.lattice"
    : > "$out/cv.gold.lattice"
    : > "$out/cv.system.lattice"
    for fold in 0 1 2 3 4 5 6 7 8 9; do
        "$libexec/cabocha-learn" -e chunk -P IPA -t UTF-8 -c "$cost" \
            "$work/folds/fold$fold.train.lattice" "$out/fold$fold.model" > "$out/fold$fold.log" 2>&1
        chunked "$out/fold$fold.model" "$work/folds/fold$fold.test.lattice" "$out/fold$fold.lattice"
        cat "$work/folds/fold$fold.test.lattice" >> "$out/cv.gold.lattice"
        cat "$out/fold$fold.lattice" >> "$out/cv.system.lattice"
        rm -f "$out/fold$fold.model" "$out/fold$fold.model.txt"
    done
    echo "cost $cost: dev $(f_score "$lattice/ja_gsd-ud-dev.lattice" "$out/dev.lattice")" \
        "test $(f_score "$lattice/ja_gsd-ud-test.lattice" "$out/test.lattice")" \
        "cross-validation $(f_score "$out/cv.gold.lattice" "$out/cv.system.lattice")"
done

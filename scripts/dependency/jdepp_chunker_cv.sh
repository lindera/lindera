#!/usr/bin/env bash
# The chunker of J.DepP in the folds of the cross-validation of
# chunker_experiments.sh: trained on fold<i>.train.lattice and run on
# fold<i>.test.lattice, for the 10 folds, with the morphemes of Lindera and
# IPADIC. What #1464 compared the chunker of lindera-dependency with.
#
# Usage: jdepp_chunker_cv.sh [--tools <tools directory>] [<data directory>]
#   --tools  Where build_baselines.sh installed J.DepP. Defaults to
#            <data directory>/tools.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The folds are those of chunker_experiments.sh,
#   <data directory>/chunker/folds, and are made when they are not there.
#   The outputs go to <data directory>/chunker/cv/jdepp: system.lattice and
#   gold.lattice, the ten folds joined, as the cross-validation of a setting
#   of lindera-dependency leaves them, so that `chunker_experiments.sh
#   jdepp` and chunker_differences.py compare the two.
#
# J.DepP is trained with the steps and the options of its own `make model`
# for its default classifier (2ndPolyFST), as in baselines.sh: its chunker
# alone. It prints one line, the F of the chunking over the ten folds.
#
# It takes about a minute.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
tools=
data=$here/data
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
        data=$1
        shift
        ;;
    esac
done
data=$(readlink -f "$data")
tools=$(readlink -f "${tools:-$data/tools}")
lattice=$data/lattice
work=$data/chunker
out=$work/cv/jdepp
format=(python3 -I -B "$here/jdepp_format.py")
jdepp=$tools/bin/jdepp

if [ ! -x "$jdepp" ]; then
    echo "$jdepp is not there; run build_baselines.sh" >&2
    exit 1
fi
if [ ! -f "$lattice/ja_gsd-ud-train.lattice" ]; then
    echo "$lattice/ja_gsd-ud-train.lattice is not there; run convert_corpora.sh" >&2
    exit 1
fi

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli
bin="$root/target/release/lindera"
if [ ! -d "$work/folds" ]; then
    mkdir -p "$work"
    "$bin" dependency split --input "$lattice/ja_gsd-ud-train.lattice" --folds 10 \
        --output-dir "$work/folds" > /dev/null
fi

# The options of J.DepP's `make model` for the classifier 2ndPolyFST.
learner=(-t 1 -d 2 -c 0.0008 -i 40 -p)
classifier=(-s 0.02 -i 5)
rm -rf "${out:?}"
mkdir -p "$out"
: > "$out/system.lattice"
: > "$out/gold.lattice"
for fold in 0 1 2 3 4 5 6 7 8 9; do
    train=$work/folds/fold$fold.train.lattice
    test=$work/folds/fold$fold.test.lattice
    model=$out/fold$fold.model
    mkdir -p "$model"
    "${format[@]}" input "$train" "$out/fold$fold.train.JDP"
    {
        "$jdepp" -t 0 -I 1 -c "$out/fold$fold.train.JDP" -m "$model" -- "${learner[@]}"
        "$jdepp" -t 3 -I 1 -c "$out/fold$fold.train.JDP" -m "$model" -- "${learner[@]}" -- "${classifier[@]}" -t 1
    } > "$out/fold$fold.learn.log" 2>&1
    "${format[@]}" input "$test" "$out/fold$fold.test.JDP"
    "$jdepp" -t 1 -I 1 -v -1 -m "$model" -- -- "${classifier[@]}" \
        < "$out/fold$fold.test.JDP" > "$out/fold$fold.out" 2> "$out/fold$fold.log"
    "${format[@]}" chunked "$out/fold$fold.out" "$test" "$out/fold$fold.out.lattice"
    cat "$out/fold$fold.out.lattice" >> "$out/system.lattice"
    cat "$test" >> "$out/gold.lattice"
    rm -r "$model" "$out/fold$fold.train.JDP" "$out/fold$fold.test.JDP" "$out/fold$fold.out" \
        "$out/fold$fold.out.lattice"
done
"$bin" dependency evaluate --gold "$out/gold.lattice" --system "$out/system.lattice" --json > "$out/evaluation.json"
python3 -I -B - "$out/evaluation.json" << 'EOF'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as file:
    chunking = json.load(file)["chunking"]
print(
    f"cv jdepp: F {100 * chunking['f_score']:.2f}"
    f" P {100 * chunking['precision']:.2f} R {100 * chunking['recall']:.2f}"
)
EOF

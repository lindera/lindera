#!/usr/bin/env bash
# The baselines of lindera-dependency: CaboCha and J.DepP trained on the
# train file of UD Japanese-GSD, on the morphemes of Lindera with IPADIC,
# and run on dev, test and UD Japanese-PUD. The numbers of "Baselines" in
# the design notes.
#
# Usage: baselines.sh [--tools <tools directory>] [--cost <cost>] [<data directory>]
#   --tools  Where build_baselines.sh installed the tools. Defaults to
#            <data directory>/tools.
#   --cost   The cost of CaboCha's chunker (`cabocha-learn -e chunk -c`).
#            Defaults to 4, the value chosen on dev.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The models and the outputs go to
#   <data directory>/baselines.
#
# For each tool and each file, one line with the three numbers counted by
# `lindera-dependency evaluate`: the dependency accuracy with the bunsetsu
# of the answer (level 1), the dependency accuracy from the morphemes (end
# to end), and the F of the chunking. Under it, what the tool reports of
# itself for the same output: `cabocha-system-eval` for CaboCha, and the
# statistics J.DepP prints. Then the arcs on which the two parsers differ,
# with the bunsetsu of the answer.
#
# CaboCha: `cabocha-learn -e chunk -c <cost>` and `cabocha-learn -e dep`,
# with its other defaults and the part-of-speech set IPA. J.DepP: the steps
# and the options of its own `make model` for its default classifier
# (2ndPolyFST), untuned. J.DepP takes a line by its first character, so its
# input and output go through jdepp_format.py.
#
# The trainings take about 20 seconds for CaboCha and 5 for J.DepP.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
tools=
cost=4
data=$here/data
while [ $# -gt 0 ]; do
    case "$1" in
    --tools)
        tools=$2
        shift 2
        ;;
    --cost)
        cost=$2
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
work=$data/baselines
files="ja_gsd-ud-dev ja_gsd-ud-test ja_pud-ud-test"
format=(python3 -I -B "$here/jdepp_format.py")
report=(python3 -I -B "$here/baseline_numbers.py")

for program in bin/cabocha libexec/cabocha/cabocha-learn libexec/cabocha/cabocha-system-eval bin/jdepp; do
    if [ ! -x "$tools/$program" ]; then
        echo "$tools/$program is not there; run build_baselines.sh" >&2
        exit 1
    fi
done
if [ ! -f "$lattice/ja_gsd-ud-train.lattice" ]; then
    echo "$lattice/ja_gsd-ud-train.lattice is not there; run convert_corpora.sh" >&2
    exit 1
fi

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features train
evaluate="$root/target/release/lindera-dependency"
train=$lattice/ja_gsd-ud-train.lattice

# numbers <label> <gold> <output with the bunsetsu of the answer> <output
# from the morphemes>: the line of `evaluate`.
numbers() {
    local label=$1 gold=$2 given=$3 end=$4
    "$evaluate" evaluate --gold "$gold" --system "$given" --json > "$given.json"
    "$evaluate" evaluate --gold "$gold" --system "$end" --json > "$end.json"
    "${report[@]}" numbers "$label" "$given.json" "$end.json"
}

# system_eval <dep|chunk> <output> <gold> <sed expression>: the line of
# cabocha-system-eval that the expression takes. The program ends with the
# status 1 when it has counted (it returns a boolean as its status), so its
# status says nothing: a count that is not printed is the failure.
system_eval() {
    local line
    line=$("$libexec/cabocha-system-eval" -e "$1" "$2" "$3" 2>&1 | sed -n "$4" || true)
    if [ -z "$line" ]; then
        echo "cabocha-system-eval -e $1 printed no count for $2" >&2
        exit 1
    fi
    echo "  cabocha-system-eval: $line"
}

# --- CaboCha ---------------------------------------------------------------
out=$work/cabocha
rm -rf "${out:?}"
mkdir -p "$out"
libexec=$tools/libexec/cabocha
# cabocha-learn writes the model it is given the name of, ready to be read,
# and its text form next to it (<name>.txt).
"$libexec/cabocha-learn" -e chunk -P IPA -t UTF-8 -c "$cost" "$train" "$out/chunk.model" > "$out/learn-chunk.log" 2>&1
"$libexec/cabocha-learn" -e dep -P IPA -t UTF-8 "$train" "$out/dep.model" > "$out/learn-dep.log" 2>&1
echo "CaboCha: chunker with the cost $cost"
for name in $files; do
    gold=$lattice/$name.lattice
    # The morphemes alone, for the chunker; the bunsetsu of the answer, for
    # the parser alone.
    grep -v -E '^\* [0-9]+ -?[0-9]+[A-Z] *$' "$gold" > "$out/$name.morphemes"
    "$tools/bin/cabocha" -I 2 -O 4 -f 1 -n 0 -P IPA -t UTF-8 -m "$out/dep.model" \
        "$gold" > "$out/$name.given.lattice"
    "$tools/bin/cabocha" -I 1 -O 4 -f 1 -n 0 -P IPA -t UTF-8 -m "$out/dep.model" \
        -M "$out/chunk.model" "$out/$name.morphemes" > "$out/$name.end.lattice"
    numbers "cabocha $name" "$gold" "$out/$name.given.lattice" "$out/$name.end.lattice"
    system_eval dep "$out/$name.given.lattice" "$gold" 's/^dependency level1: */level 1 /p'
    system_eval chunk "$out/$name.end.lattice" "$gold" 's/^Chunk: */chunking P, R, F /p'
done

# --- J.DepP ----------------------------------------------------------------
out=$work/jdepp
rm -rf "${out:?}"
mkdir -p "$out/model"
jdepp=$tools/bin/jdepp
# The options of J.DepP's `make model` for the classifier 2ndPolyFST.
chunker_learner=(-t 1 -d 2 -c 0.0008 -i 40 -p)
chunker_classifier=(-s 0.02 -i 5)
parser_learner=(-t 1 -d 2 -c 0.00005 -i 40 -p)
parser_classifier=(-s 0.005 -i 5)
"${format[@]}" input "$train" "$out/train.JDP"
{
    "$jdepp" -t 0 -I 1 -c "$out/train.JDP" -m "$out/model" -- "${chunker_learner[@]}"
    "$jdepp" -t 3 -I 1 -c "$out/train.JDP" -m "$out/model" -- "${chunker_learner[@]}" -- "${chunker_classifier[@]}" -t 1
    "$jdepp" -t 0 -I 2 -c "$out/train.JDP" -m "$out/model" -- "${parser_learner[@]}"
    "$jdepp" -t 3 -I 2 -c "$out/train.JDP" -m "$out/model" -- "${parser_learner[@]}" -- -- "${parser_classifier[@]}" -t 1
} > "$out/learn.log" 2>&1
echo "J.DepP: the options of its make model (2ndPolyFST)"
for name in $files; do
    gold=$lattice/$name.lattice
    "${format[@]}" input "$gold" "$out/$name.JDP"
    "${format[@]}" input "$gold" "$out/$name.morphemes" --morphemes
    "$jdepp" -t 1 -I 1 -v -1 -m "$out/model" -- -- "${chunker_classifier[@]}" \
        < "$out/$name.JDP" > "$out/$name.chunked.out" 2> "$out/$name.chunked.log"
    "$jdepp" -t 1 -I 2 -v -1 -m "$out/model" -- -- -- "${parser_classifier[@]}" \
        < "$out/$name.JDP" > "$out/$name.given.out" 2> "$out/$name.given.log"
    "$jdepp" -t 1 -I 0 -m "$out/model" -- -- "${chunker_classifier[@]}" -- "${parser_classifier[@]}" \
        < "$out/$name.morphemes" > "$out/$name.end.out" 2> "$out/$name.end.log"
    "${format[@]}" parsed "$out/$name.given.out" "$gold" "$out/$name.given.lattice"
    "${format[@]}" parsed "$out/$name.end.out" "$gold" "$out/$name.end.lattice"
    "${format[@]}" chunked "$out/$name.chunked.out" "$gold" "$out/$name.chunked.lattice"
    numbers "jdepp $name" "$gold" "$out/$name.given.lattice" "$out/$name.end.lattice"
    sed -n 's/^acc\. (partial)[[:space:]]*/  jdepp: level 1 /p' "$out/$name.given.log"
    tr '\n' ' ' < "$out/$name.chunked.log" |
        sed -n 's/.*precision[[:space:]]*\([^)]*)\) recall[[:space:]]*\([^)]*)\) f1[[:space:]]*\([0-9.]*\).*/  jdepp: chunking P \1, R \2, F \3\n/p'
    # The chunker alone and the chunker before the parser are the same.
    "$evaluate" evaluate --gold "$out/$name.chunked.lattice" --system "$out/$name.end.lattice" \
        --json > "$out/$name.chunkers.json"
    "${report[@]}" same-bunsetsu "$out/$name.chunkers.json"
done

# --- The two parsers ---------------------------------------------------------
for name in $files; do
    "$evaluate" evaluate --gold "$work/cabocha/$name.given.lattice" \
        --system "$work/jdepp/$name.given.lattice" --json > "$work/parsers.$name.json"
    "${report[@]}" differ "cabocha and jdepp $name" "$work/parsers.$name.json"
done

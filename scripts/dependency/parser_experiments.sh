#!/usr/bin/env bash
# The experiments behind the defaults of the parser of lindera-dependency:
# the numbers of "How the defaults were chosen" of the parser in its README
# and of the design notes.
#
# Usage: parser_experiments.sh [--dictionary <name>] <stage>... [-- <data directory>]
#   --dictionary  ipadic (the default), unidic, unidic-cwj or unidic-csj:
#                 the dictionary convert_corpora.sh was run with.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The outputs go to <data directory>/parser. For
#   another dictionary than ipadic, both are in <data directory>/<name>.
#
# Stages, each one line of result per setting. A cross-validation is over
# the 10 folds of the train file, on the bunsetsu of the answer.
#   algorithm     Cross-validation of dual coordinate descent and of
#                 passive-aggressive over their settings, with CaboCha's
#                 feature set and a weight for every pair.
#   features      Cross-validation of feature sets: CaboCha's, the same
#                 without each of its groups, and with the morpheme after
#                 the head.
#   pairs         Cross-validation of which pairs get a weight: a minimum
#                 support, a threshold on the weight, a minimum count of
#                 the features.
#   storage       Cross-validation of tables the pairs are hashed into,
#                 from 2^16 to 2^24 weights; and four of them on the whole
#                 train file, evaluated on dev.
#   quantization  Cross-validation of the default with weights of 2 bytes
#                 and of 1 byte, and the three on the whole train file,
#                 evaluated on dev and test.
#   compare       Pairs of settings of the cross-validations, by the arcs,
#                 with McNemar's exact test. Needs the stages above.
#   final         The defaults of `train-parser` on the whole train file,
#                 with the chunker, evaluated on dev, test and PUD on the
#                 bunsetsu of the answer and from the morphemes; trained
#                 twice to see the same bytes. CaboCha's feature set the
#                 same way, to set next to it.
#   speed         The size of the file, the time to read it and the
#                 sentences per second, with model_speed/, for the default
#                 and for the other tables of the pairs.
#   all           Every stage, in this order.
#
# A cross-validation trains ten models at the same time and takes about ten
# seconds; all the stages take about ten minutes.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
# shellcheck source=scripts/dependency/dictionary.sh
. "$here/dictionary.sh"
parse_dictionary_arguments "$@"
set -- "${rest[@]}"
data=$here/data
stages=()
while [ $# -gt 0 ]; do
    if [ "$1" = "--" ]; then
        data=$2
        break
    fi
    stages+=("$1")
    shift
done
data=$(dictionary_directory "$(readlink -f "$data")")
# The option of the training commands that names the dictionary; none for
# ipadic, which is their default.
named=()
if [ "$dictionary" != ipadic ]; then
    named=(--dictionary-name "$dictionary")
fi
lattice=$data/lattice
work=$data/parser
experiment=(python3 -I -B "$here/parser_experiment.py")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features train
bin="$root/target/release/lindera-dependency"
mkdir -p "$work"

# The feature set of CaboCha, and the same with the morpheme after the head.
cabocha=distance,source,target,left,right,children,gap
all=$cabocha,after
files=ja_gsd-ud-dev,ja_gsd-ud-test,ja_pud-ud-test

# settings <option> <value>...: puts the options of train-parser into
# `options`. Every stage starts from the same settings (CaboCha's feature
# set, dual coordinate descent with the cost 0.0015, a weight for every
# pair, weights of 4 bytes) and gives the ones it varies.
settings() {
    local -A values=([--algorithm]=dcd [--cost]=0.0015 [--features]="$cabocha"
        [--pairs]=exact [--min-support]=1 [--min-count]=1 [--pair-threshold]=0
        [--quantization]=f32 [--seed]=1)
    local averaged="" key
    while [ $# -gt 0 ]; do
        if [ "$1" = --averaged ]; then
            averaged=$1
            shift
        else
            values[$1]=$2
            shift 2
        fi
    done
    options=()
    for key in --algorithm --cost --iterations --features --pairs --bits --min-support \
        --min-count --pair-threshold --quantization --seed; do
        if [ -n "${values[$key]:-}" ]; then
            options+=("$key" "${values[$key]}")
        fi
    done
    if [ -n "$averaged" ]; then
        options+=("$averaged")
    fi
    options+=("${named[@]}")
}

# cross_validate <name> <train-parser options...>
cross_validate() {
    local name=$1
    shift
    if [ ! -d "$work/folds" ]; then
        "$bin" split --input "$lattice/ja_gsd-ud-train.lattice" --folds 10 \
            --output-dir "$work/folds" > /dev/null
    fi
    settings "$@"
    "${experiment[@]}" cv "$bin" "$work/folds" "$work" "$name" "${options[@]}"
}

# whole <name> <files> <train-parser options...>: trains on the whole train
# file, with the chunker, and evaluates on the files.
whole() {
    local name=$1 evaluated=$2
    shift 2
    settings "$@"
    "${experiment[@]}" full "$bin" "$lattice" "$work" "$name" "$evaluated" "${options[@]}"
}

algorithm() {
    local cost iterations
    for cost in 0.0005 0.001 0.0015 0.002 0.003 0.005 0.01; do
        cross_validate "dcd-$cost" --cost "$cost"
    done
    for cost in 0.0005 0.001 0.005; do
        for iterations in 10 30; do
            cross_validate "pa-$cost-$iterations" \
                --algorithm pa --cost "$cost" --iterations "$iterations"
            cross_validate "pa-$cost-$iterations-averaged" \
                --algorithm pa --cost "$cost" --iterations "$iterations" --averaged
        done
    done
}

features() {
    local group set
    cross_validate cabocha
    for group in distance left right children gap; do
        set=$(echo "$cabocha" | sed "s/$group,//; s/,$group\$//")
        cross_validate "cabocha-without-$group" --features "$set"
    done
    cross_validate all --features "$all"
    cross_validate all-without-children \
        --features distance,source,target,left,right,gap,after
}

pairs() {
    local value
    for value in 2 3 5 10; do
        cross_validate "support-$value" --features "$all" --min-support "$value"
    done
    for value in 0.0005 0.001 0.002; do
        cross_validate "threshold-$value" --features "$all" --pair-threshold "$value"
    done
    for value in 2 3 5; do
        cross_validate "count-$value" --features "$all" --min-count "$value"
    done
}

storage() {
    local bits
    for bits in 16 18 20 21 22 23 24; do
        cross_validate "hashed-$bits" --features "$all" --pairs hashed --bits "$bits"
    done
    # The sizes the cross-validation does not tell apart, on the dev file,
    # which chooses between close settings.
    for bits in 18 20 21 22; do
        whole "hashed-$bits-i8" ja_gsd-ud-dev --features "$all" \
            --pairs hashed --bits "$bits" --quantization i8
    done
}

quantization() {
    local kind
    for kind in i16 i8; do
        cross_validate "hashed-22-$kind" --features "$all" \
            --pairs hashed --bits 22 --quantization "$kind"
    done
    for kind in f32 i16 i8; do
        whole "hashed-22-$kind" ja_gsd-ud-dev,ja_gsd-ud-test --features "$all" \
            --pairs hashed --bits 22 --quantization "$kind"
    done
}

compare() {
    local pair
    # `all` is every group of features, dual coordinate descent with the
    # cost 0.0015 and a weight for every pair; `cabocha` the same with
    # CaboCha's feature set, which is also `dcd-0.0015`.
    for pair in \
        dcd-0.0015:dcd-0.001 \
        dcd-0.0015:dcd-0.003 \
        dcd-0.0015:pa-0.0005-30-averaged \
        cabocha:cabocha-without-children \
        cabocha:cabocha-without-right \
        cabocha:cabocha-without-gap \
        cabocha:all \
        all:all-without-children \
        all:support-3 \
        all:count-3 \
        all:threshold-0.002 \
        all:hashed-18 \
        all:hashed-20 \
        all:hashed-21 \
        all:hashed-22 \
        hashed-22:hashed-20 \
        hashed-22:hashed-22-i16 \
        hashed-22:hashed-22-i8; do
        "${experiment[@]}" compare "$bin" "$work" "${pair%%:*}" "${pair##*:}"
    done
}

final() {
    # No option: the defaults of the command.
    "${experiment[@]}" full "$bin" "$lattice" "$work" default "$files" "${named[@]}"
    "$bin" train-parser --input "$lattice/ja_gsd-ud-train.lattice" \
        --chunker "$work/full/default/chunker.model" \
        --output "$work/full/default/again" "${named[@]}" 2> /dev/null
    if cmp -s "$work/full/default/model" "$work/full/default/again"; then
        echo "default: trained twice, the same bytes"
    else
        echo "default: trained twice, DIFFERENT bytes"
        exit 1
    fi
    "${experiment[@]}" full "$bin" "$lattice" "$work" default-cabocha "$files" \
        --features "$cabocha" "${named[@]}"
}

speed() {
    local name
    cargo build --release --manifest-path "$here/model_speed/Cargo.toml"
    "${experiment[@]}" full "$bin" "$lattice" "$work" default "$files" "${named[@]}" > /dev/null
    whole speed-hashed-22-f32 ja_gsd-ud-dev --features "$all" --pairs hashed --bits 22 > /dev/null
    whole speed-exact-i8 ja_gsd-ud-dev --features "$all" --quantization i8 > /dev/null
    whole speed-exact-f32 ja_gsd-ud-dev --features "$all" > /dev/null
    for name in default speed-hashed-22-f32 speed-exact-i8 speed-exact-f32; do
        echo "speed $name:"
        "$here/model_speed/target/release/model_speed" "$work/full/$name/model" \
            "$lattice/ja_gsd-ud-test.lattice" 7 | sed 's/^/  /'
    done
}

for stage in "${stages[@]}"; do
    case "$stage" in
    algorithm | features | pairs | storage | quantization | compare | final | speed) "$stage" ;;
    all)
        algorithm
        features
        pairs
        storage
        quantization
        compare
        final
        speed
        ;;
    *)
        echo "unknown stage $stage" >&2
        exit 1
        ;;
    esac
done

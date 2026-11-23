#!/usr/bin/env bash
# The experiments behind the defaults of the chunker of lindera-dependency:
# the numbers of "How the defaults were chosen" in its README.
#
# Usage: chunker_experiments.sh <stage>... [-- <data directory>]
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The outputs go to <data directory>/chunker.
#
# Stages, each one line of result per setting:
#   table    CaboCha's template on the whole train file, with L2 and L1 and
#            the coefficients 0.25 and 1.0, evaluated on dev and test.
#   cv       10-fold cross-validation on train: six sets of columns, each
#            with L1 and L2 and the coefficients 0.25, 0.5 and 1.0.
#   small    The cross-validation with smaller coefficients of L1, and with
#            the conjugation form and the base form together.
#   compare  Pairs of settings of the cross-validation, by the segmentation
#            of each sentence, with McNemar's exact test. Needs cv and small.
#   default  The defaults of `train-chunker` on the whole train file,
#            evaluated on dev, test and PUD, and trained twice to see the
#            same bytes.
#   all      Every stage, in this order.
#
# The cross-validation trains ten models per setting: cv and small take
# about an hour together.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
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
data=$(readlink -f "$data")
lattice=$data/lattice
work=$data/chunker
experiment=(python3 -I -B "$here/chunker_experiment.py")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features train
bin="$root/target/release/lindera-dependency"
mkdir -p "$work"
python3 -I -B "$here/make_templates.py" "$work/templates"

# The columns and the template of a set of columns.
columns_of() {
    case "$1" in
    cabocha) echo "surface,pos cabocha" ;;
    cform) echo "surface,pos,cform extra" ;;
    ctype) echo "surface,pos,ctype extra" ;;
    base) echo "surface,pos,base extra" ;;
    cform-light) echo "surface,pos,cform light" ;;
    base-light) echo "surface,pos,base light" ;;
    cform-base) echo "surface,pos,cform,base both" ;;
    *)
        echo "unknown set of columns $1" >&2
        exit 1
        ;;
    esac
}

# cross_validate <set of columns> <l1|l2> <coefficient>
cross_validate() {
    local columns template
    read -r columns template <<< "$(columns_of "$1")"
    if [ ! -d "$work/folds" ]; then
        "$bin" split --input "$lattice/ja_gsd-ud-train.lattice" --folds 10 \
            --output-dir "$work/folds" > /dev/null
    fi
    "${experiment[@]}" cv "$bin" "$work/folds" "$work" "$1-$2-$3" \
        --regularization "$2" --lambda "$3" --columns "$columns" \
        --template "$work/templates/$template.template"
}

table() {
    local regularization coefficient
    for setting in "l2 0.25" "l2 1.0" "l1 0.25" "l1 1.0"; do
        read -r regularization coefficient <<< "$setting"
        "${experiment[@]}" full "$bin" "$lattice" "$work" \
            "cabocha-$regularization-$coefficient" ja_gsd-ud-dev,ja_gsd-ud-test \
            --regularization "$regularization" --lambda "$coefficient" \
            --columns surface,pos --template "$work/templates/cabocha.template"
    done
}

cv() {
    local regularization coefficient
    for columns in cabocha cform cform-light ctype base base-light; do
        for setting in "l1 0.25" "l1 0.5" "l1 1.0" "l2 0.25" "l2 0.5" "l2 1.0"; do
            read -r regularization coefficient <<< "$setting"
            cross_validate "$columns" "$regularization" "$coefficient"
        done
    done
}

small() {
    for columns in cabocha cform; do
        for coefficient in 0.1 0.15; do
            cross_validate "$columns" l1 "$coefficient"
        done
    done
    for coefficient in 0.1 0.15 0.25; do
        cross_validate cform-base l1 "$coefficient"
    done
}

compare() {
    # The default setting is cform-light-l1-0.5.
    for pair in \
        cabocha-l1-0.25:cform-l1-0.25 \
        cabocha-l1-0.25:ctype-l1-0.25 \
        cabocha-l1-0.25:base-l1-0.25 \
        cform-l1-0.25:cform-base-l1-0.25 \
        cform-l1-0.25:cform-l1-0.5 \
        cform-l1-0.25:cform-l1-1.0 \
        cform-l1-0.25:cform-l2-0.25 \
        cform-l1-0.25:cform-light-l1-0.5 \
        cabocha-l1-0.25:cform-light-l1-0.5 \
        cform-light-l1-0.5:cform-light-l1-1.0; do
        "${experiment[@]}" compare "$bin" "$work" "${pair%%:*}" "${pair##*:}"
    done
}

default() {
    "${experiment[@]}" full "$bin" "$lattice" "$work" default \
        ja_gsd-ud-dev,ja_gsd-ud-test,ja_pud-ud-test
    "$bin" train-chunker --input "$lattice/ja_gsd-ud-train.lattice" \
        --output "$work/full/default/again.model" 2> /dev/null
    if cmp -s "$work/full/default/chunker.model" "$work/full/default/again.model"; then
        echo "default: trained twice, the same bytes"
    else
        echo "default: trained twice, DIFFERENT bytes"
        exit 1
    fi
}

for stage in "${stages[@]}"; do
    case "$stage" in
    table | cv | small | compare | default) "$stage" ;;
    all)
        table
        cv
        small
        compare
        default
        ;;
    *)
        echo "unknown stage $stage" >&2
        exit 1
        ;;
    esac
done

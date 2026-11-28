#!/usr/bin/env bash
# The experiments behind the defaults of the chunker of lindera-dependency:
# the numbers of "How the defaults were chosen" in its README.
#
# Usage: chunker_experiments.sh [--dictionary <name>] <stage>... [-- <data directory>]
#   --dictionary  ipadic (the default), unidic, unidic-cwj or unidic-csj:
#                 the dictionary convert_corpora.sh was run with.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The outputs go to <data directory>/chunker. For
#   another dictionary than ipadic, both are in <data directory>/<name>.
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
#   base     The cross-validation of the base form with smaller coefficients
#            of L1, and the pairs of settings around the base form: what the
#            settings for UniDic, whose base form is the lexeme, rest on.
#            Needs cv and small.
#   check    The cross-validation of the setting for UniDic and of four
#            settings next to it, and their pairs: for a dictionary the
#            other stages were not run for. Needs no other stage.
#   default  The defaults of `train-chunker` on the whole train file,
#            evaluated on dev, test and PUD, and trained twice to see the
#            same bytes.
#   jdepp    What #1464 measured: the cross-validation of the default
#            setting and of three templates with the combinations J.DepP's
#            chunker has, their pairs with the default, and the pair of
#            the default with J.DepP's own chunker in the same folds, when
#            jdepp_chunker_cv.sh was run. Needs no other stage, and is not
#            part of `all`.
#   all      Every stage but jdepp, in this order.
#
# The cross-validation trains ten models per setting: cv and small take
# about an hour together.
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
work=$data/chunker
experiment=(python3 -I -B "$here/chunker_experiment.py")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli
bin="$root/target/release/lindera"
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
    cross-pos) echo "surface,pos,cform cross-pos" ;;
    cross-form) echo "surface,pos,cform cross-form" ;;
    cross) echo "surface,pos,cform cross" ;;
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
        "$bin" dependency split --input "$lattice/ja_gsd-ud-train.lattice" --folds 10 \
            --output-dir "$work/folds" > /dev/null
    fi
    "${experiment[@]}" cv "$bin" "$work/folds" "$work" "$1-$2-$3" \
        --regularization "$2" --lambda "$3" --columns "$columns" \
        --template "$work/templates/$template.template" "${named[@]}"
}

table() {
    local regularization coefficient
    for setting in "l2 0.25" "l2 1.0" "l1 0.25" "l1 1.0"; do
        read -r regularization coefficient <<< "$setting"
        "${experiment[@]}" full "$bin" "$lattice" "$work" \
            "cabocha-$regularization-$coefficient" ja_gsd-ud-dev,ja_gsd-ud-test \
            --regularization "$regularization" --lambda "$coefficient" \
            --columns surface,pos --template "$work/templates/cabocha.template" \
            "${named[@]}"
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

base() {
    for coefficient in 0.1 0.15; do
        cross_validate base l1 "$coefficient"
    done
    # cform-light-l1-0.5 is the setting for IPADIC.
    for pair in \
        cform-light-l1-0.5:base-l1-0.25 \
        cform-light-l1-0.5:base-l1-0.5 \
        cform-light-l1-0.5:base-light-l1-0.25 \
        base-l1-0.25:base-l1-0.1 \
        base-l1-0.25:base-l1-0.5 \
        base-l1-0.25:base-l1-1.0 \
        base-l1-0.25:base-l2-0.25 \
        base-l1-0.25:base-light-l1-0.25 \
        base-l1-0.25:cform-base-l1-0.25; do
        "${experiment[@]}" compare "$bin" "$work" "${pair%%:*}" "${pair##*:}"
    done
}

check() {
    local columns regularization coefficient
    for setting in "cabocha l1 0.25" "cform-light l1 0.5" "cform l1 0.25" \
        "base l1 0.25" "cform-base l1 0.25"; do
        read -r columns regularization coefficient <<< "$setting"
        cross_validate "$columns" "$regularization" "$coefficient"
    done
    # cform-light-l1-0.5 is the setting for IPADIC, cform-base-l1-0.25 the
    # one for UniDic.
    for pair in \
        cabocha-l1-0.25:cform-base-l1-0.25 \
        cform-light-l1-0.5:cform-base-l1-0.25 \
        cform-l1-0.25:cform-base-l1-0.25 \
        base-l1-0.25:cform-base-l1-0.25; do
        "${experiment[@]}" compare "$bin" "$work" "${pair%%:*}" "${pair##*:}"
    done
}

jdepp() {
    local columns regularization coefficient
    # cform-light-l1-0.5 is the default setting. A template with more
    # lines has more weights to hold down, so the largest one is tried
    # with a larger coefficient too.
    for setting in "cform-light l1 0.5" "cross-pos l1 0.5" "cross-form l1 0.5" \
        "cross l1 0.5" "cross l1 1.0"; do
        read -r columns regularization coefficient <<< "$setting"
        cross_validate "$columns" "$regularization" "$coefficient"
    done
    for other in cross-pos-l1-0.5 cross-form-l1-0.5 cross-l1-0.5 cross-l1-1.0; do
        "${experiment[@]}" compare "$bin" "$work" cform-light-l1-0.5 "$other"
    done
    # The chunker of J.DepP, which jdepp_chunker_cv.sh trains and runs in
    # the same folds.
    if [ -f "$work/cv/jdepp/system.lattice" ]; then
        "${experiment[@]}" compare "$bin" "$work" cform-light-l1-0.5 jdepp
    else
        echo "jdepp: $work/cv/jdepp is not there; run jdepp_chunker_cv.sh for the pair with J.DepP"
    fi
}

default() {
    "${experiment[@]}" full "$bin" "$lattice" "$work" default \
        ja_gsd-ud-dev,ja_gsd-ud-test,ja_pud-ud-test "${named[@]}"
    "$bin" dependency train-chunker --input "$lattice/ja_gsd-ud-train.lattice" \
        --output "$work/full/default/again.model" "${named[@]}" 2> /dev/null
    if cmp -s "$work/full/default/chunker.model" "$work/full/default/again.model"; then
        echo "default: trained twice, the same bytes"
    else
        echo "default: trained twice, DIFFERENT bytes"
        exit 1
    fi
}

for stage in "${stages[@]}"; do
    case "$stage" in
    table | cv | small | compare | base | check | default | jdepp) "$stage" ;;
    all)
        table
        cv
        small
        compare
        base
        default
        ;;
    *)
        echo "unknown stage $stage" >&2
        exit 1
        ;;
    esac
done

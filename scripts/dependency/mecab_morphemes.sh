#!/usr/bin/env bash
# The agreement of the morphemes of Lindera with those of MeCab, for a
# dictionary both have: the same text is tokenized by `mecab -d` with the
# dictionary build_baselines.sh installed, and by `lindera tokenize` with
# the dictionary of Lindera embedded.
#
# Usage: mecab_morphemes.sh [--tools <tools directory>] [--dictionary <name>] [--examples <count>] [<data directory>]
#   --tools       Where build_baselines.sh installed MeCab and its
#                 dictionaries. Defaults to <data directory>/tools.
#   --dictionary  jumandic (the default) or ipadic.
#   --examples    How many first differences of each kind to print.
#                 Defaults to 0.
#   <data directory> defaults to scripts/dependency/data and holds gsd/ and
#   pud/ (fetch_corpora.sh). The texts and the outputs go to
#   <data directory>/mecab/<name>.
#
# The text is the sentences of UD Japanese-GSD (train, dev and test) and of
# UD Japanese-PUD, as their `# text =` lines give them.
#
# For each file it prints the sentences with the same segmentation, and of
# those the sentences whose morphemes also have the same feature columns:
# for jumandic the first six (the part of speech, the conjugation, the base
# form and the reading) and all seven (with the semantic information); for
# ipadic the first seven and all nine. Then what the first difference of a
# sentence is: the kind of the characters where the segmentation differs,
# and the column where only the features differ. A second line has the same
# counts for `lindera tokenize --disable-break-characters`: by default
# Lindera lets no unknown word cross a `・`, which MeCab does not know of.
#
# MeCab's dictionaries are built from the repository of MeCab: mecab-ipadic
# 2.7.0-20070801 and mecab-jumandic 7.0-20130310. Lindera's jumandic is
# built from the same source; its IPADIC is mecab-ipadic 2.7.0-20250920.
#
# It takes less than a minute.
set -euo pipefail
here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
tools=
dictionary=jumandic
examples=0
data=$here/data
while [ $# -gt 0 ]; do
    case "$1" in
    --tools)
        tools=$2
        shift 2
        ;;
    --dictionary)
        dictionary=$2
        shift 2
        ;;
    --examples)
        examples=$2
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
case "$dictionary" in
jumandic) columns="6 7" ;;
ipadic) columns="7 9" ;;
*)
    echo "unknown dictionary $dictionary: expected jumandic or ipadic" >&2
    exit 2
    ;;
esac
data=$(readlink -f "$data")
tools=$(readlink -f "${tools:-$data/tools}")
work=$data/mecab/$dictionary
files="gsd/ja_gsd-ud-dev gsd/ja_gsd-ud-test pud/ja_pud-ud-test gsd/ja_gsd-ud-train"

if [ ! -x "$tools/bin/mecab" ] || [ ! -s "$tools/lib/mecab/dic/$dictionary/sys.dic" ]; then
    echo "$tools has no MeCab with the dictionary $dictionary; run build_baselines.sh" >&2
    exit 1
fi
for file in $files; do
    if [ ! -f "$data/$file.conllu" ]; then
        echo "$data/$file.conllu is not there; run fetch_corpora.sh" >&2
        exit 1
    fi
done

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli \
    --features "embed-$dictionary"
rm -rf "${work:?}"
mkdir -p "$work"
# The words of the two outputs are compared as they are written.
# shellcheck disable=SC2086
for file in $files; do
    name=$(basename "$file")
    sed -n 's/^# text = //p' "$data/$file.conllu" > "$work/$name.txt"
    "$tools/bin/mecab" -d "$tools/lib/mecab/dic/$dictionary" "$work/$name.txt" > "$work/$name.mecab"
    "$root/target/release/lindera" tokenize --dict "embedded://$dictionary" \
        "$work/$name.txt" > "$work/$name.lindera"
    python3 -I -B "$here/mecab_agreement.py" "$name" "$work/$name.mecab" "$work/$name.lindera" \
        $columns --examples "$examples"
    "$root/target/release/lindera" tokenize --dict "embedded://$dictionary" \
        --disable-break-characters "$work/$name.txt" > "$work/$name.unbroken.lindera"
    python3 -I -B "$here/mecab_agreement.py" "$name, without break characters" \
        "$work/$name.mecab" "$work/$name.unbroken.lindera" $columns | sed -n 1,2p
done

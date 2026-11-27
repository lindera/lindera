#!/usr/bin/env bash
# The agreement of a converted model of CaboCha with CaboCha: a model that
# CaboCha distributes, for IPA or for JUMAN, is converted with
# `lindera-dependency convert-cabocha`, and the same input is given to
# CaboCha, with that model as its own build compiled it, and to
# `lindera-dependency parse` with the converted one. The numbers of
# "CaboCha's models" in the documentation.
#
# Usage: cabocha_models.sh [--tools <tools directory>] [--posset <set>] [<data directory>]
#   --tools   Where build_baselines.sh installed the tools, and left the
#             source of CaboCha with its models in text. Defaults to
#             <data directory>/tools.
#   --posset  The part-of-speech set of the model: ipa (the default) or
#             juman.
#   <data directory> defaults to scripts/dependency/data. The converted
#   models and the outputs go to <data directory>/cabocha for ipa, and to
#   <data directory>/cabocha-juman for juman.
#
# The input is the sentences of UD Japanese-GSD and of UD Japanese-PUD:
#
# - for ipa, with the morphemes of Lindera with IPADIC, from
#   <data directory>/lattice (convert_corpora.sh);
# - for juman, with the morphemes of MeCab with mecab-jumandic, which
#   build_baselines.sh installed, for the text of the corpus files in
#   <data directory>/gsd and <data directory>/pud (fetch_corpora.sh).
#
# The models of CaboCha are not under the license of CaboCha: its page
# limits the use of the distributed models as they are to research and
# personal use. They stay in the tools directory, and the converted models
# in the data directory, which git ignores. Do not distribute either.
#
# It prints:
#
# - what `convert-cabocha` reports, with the defaults (CaboCha's
#   approximation) and with `--sigma 0 --min-support 1` (the kernel
#   itself), and the size of the two files;
# - for each of the four files, the agreement on the same morphemes: the
#   sentences with the same bunsetsu, the bunsetsu with the same head and
#   with the same head word and function word, and the largest difference
#   of a score; with the defaults, then with the kernel itself;
# - the agreement on the same text, where CaboCha tokenizes with MeCab and
#   its dictionary, and Lindera with its own: `parse` with IPADIC for ipa,
#   and `lindera tokenize` with the JUMAN dictionary, whose morphemes
#   `parse` is given, for juman;
# - the time of `parse` and of CaboCha on the morphemes of the train file.
#
# The bunsetsu of the models are those of the Kyoto University Text Corpus,
# not those of UD Japanese-GSD: the files are a fixed text here, and their
# answers are not used.
#
# It takes about a minute for a set.
set -euo pipefail
here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
tools=
posset=ipa
data=$here/data
while [ $# -gt 0 ]; do
    case "$1" in
    --tools)
        tools=$2
        shift 2
        ;;
    --posset)
        posset=$2
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
case "$posset" in
ipa)
    set_name=IPA
    dictionary=ipadic
    columns=7
    ;;
juman)
    set_name=JUMAN
    dictionary=jumandic
    columns=5
    ;;
*)
    echo "unknown part-of-speech set $posset: expected ipa or juman" >&2
    exit 2
    ;;
esac
data=$(readlink -f "$data")
tools=$(readlink -f "${tools:-$data/tools}")
lattice=$data/lattice
if [ "$posset" = ipa ]; then
    work=$data/cabocha
else
    work=$data/cabocha-$posset
fi
files="ja_gsd-ud-dev ja_gsd-ud-test ja_pud-ud-test ja_gsd-ud-train"
agreement=(python3 -I -B "$here/cabocha_agreement.py")
text_models=$tools/build/cabocha/model
compiled=$tools/lib/cabocha/model
mecab_dictionary=$tools/lib/mecab/dic/$dictionary

for file in "$tools/bin/cabocha" "$text_models/chunk.$posset.txt" "$text_models/dep.$posset.txt" \
    "$compiled/chunk.$posset.model" "$compiled/dep.$posset.model" "$mecab_dictionary/sys.dic"; do
    if [ ! -s "$file" ]; then
        echo "$file is not there; run build_baselines.sh" >&2
        exit 1
    fi
done
# The corpus file of a name: ja_pud-ud-test is in pud, the others in gsd.
corpus() {
    case "$1" in
    ja_pud-*) echo "$data/pud/$1.conllu" ;;
    *) echo "$data/gsd/$1.conllu" ;;
    esac
}
for name in $files; do
    if [ "$posset" = ipa ] && [ ! -f "$lattice/$name.lattice" ]; then
        echo "$lattice/$name.lattice is not there; run convert_corpora.sh" >&2
        exit 1
    fi
    if [ "$posset" != ipa ] && [ ! -f "$(corpus "$name")" ]; then
        echo "$(corpus "$name") is not there; run fetch_corpora.sh" >&2
        exit 1
    fi
done

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency \
    --features train,embed-ipadic
command="$root/target/release/lindera-dependency"
if [ "$posset" != ipa ]; then
    # `lindera-dependency` embeds IPADIC alone: the text is tokenized by
    # the `lindera` command with the dictionary embedded.
    cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli \
        --features "embed-$dictionary"
fi
rm -rf "${work:?}"
mkdir -p "$work"

# CaboCha with its models for the set, without named entities, writing the
# lattice format: <input layer> <input>. For text it tokenizes with MeCab
# and the dictionary of the set.
cabocha() {
    "$tools/bin/cabocha" -I "$1" -O 4 -f 1 -n 0 -P "$set_name" -t UTF-8 \
        -d "$mecab_dictionary" \
        -m "$compiled/dep.$posset.model" -M "$compiled/chunk.$posset.model" "$2"
}

# convert <output> <option>...: the report of `convert-cabocha`, without
# its seconds and its last line, which says the terms of the models.
convert() {
    local output=$1
    shift
    "$command" convert-cabocha --chunker "$text_models/chunk.$posset.txt" \
        --parser "$text_models/dep.$posset.txt" --output "$output" "$@" 2>&1 |
        grep -v -e '^seconds: ' -e '^The models of CaboCha '
}

echo "--- convert-cabocha, the defaults"
convert "$work/cabocha-$posset.model"
echo "--- convert-cabocha --sigma 0 --min-support 1"
convert "$work/cabocha-$posset.kernel.model" --sigma 0 --min-support 1

echo "--- on the same morphemes"
for name in $files; do
    if [ "$posset" = ipa ]; then
        # The morphemes alone: the bunsetsu of the answer are those of UD
        # Japanese-GSD.
        grep -v -E '^\* [0-9]+ -?[0-9]+[A-Z] *$' "$lattice/$name.lattice" > "$work/$name.morphemes"
    else
        sed -n 's/^# text = //p' "$(corpus "$name")" > "$work/$name.txt"
        "$tools/bin/mecab" -d "$mecab_dictionary" "$work/$name.txt" > "$work/$name.morphemes"
    fi
    cabocha 1 "$work/$name.morphemes" > "$work/$name.cabocha.lattice"
    "$command" parse --input-layer morphemes --model "$work/cabocha-$posset.model" \
        --input "$work/$name.morphemes" --output "$work/$name.parse.lattice"
    "${agreement[@]}" outputs "$name" "$work/$name.cabocha.lattice" \
        "$work/$name.parse.lattice" --differences "$work/$name.differences"
done
echo "--- on the same morphemes, the kernel itself against CaboCha"
for name in $files; do
    "$command" parse --input-layer morphemes --model "$work/cabocha-$posset.kernel.model" \
        --input "$work/$name.morphemes" --output "$work/$name.kernel.lattice"
    "${agreement[@]}" outputs "$name" "$work/$name.cabocha.lattice" \
        "$work/$name.kernel.lattice" --differences "$work/$name.kernel.differences"
done

if [ "$posset" = ipa ]; then
    echo "--- on the same text: MeCab with mecab-ipadic, and Lindera with IPADIC"
else
    echo "--- on the same text: MeCab with mecab-$dictionary, and Lindera with $dictionary"
fi
for name in $files; do
    if [ "$posset" = ipa ]; then
        "${agreement[@]}" sentences "$lattice/$name.lattice" "$work/$name.txt"
    fi
    cabocha 0 "$work/$name.txt" > "$work/$name.cabocha.text.lattice"
    if [ "$posset" = ipa ]; then
        "$command" parse --input-layer text --dict embedded://ipadic \
            --model "$work/cabocha-$posset.model" --input "$work/$name.txt" \
            --output "$work/$name.parse.text.lattice"
    else
        "$root/target/release/lindera" tokenize --dict "embedded://$dictionary" \
            "$work/$name.txt" > "$work/$name.lindera.morphemes"
        "$command" parse --input-layer morphemes --model "$work/cabocha-$posset.model" \
            --input "$work/$name.lindera.morphemes" --output "$work/$name.parse.text.lattice"
    fi
    "${agreement[@]}" text "$name" "$work/$name.cabocha.text.lattice" \
        "$work/$name.parse.text.lattice" --columns "$columns"
done

echo "--- seconds for the morphemes of ja_gsd-ud-train, the best of three"
name=ja_gsd-ud-train
seconds() {
    local best= start end run elapsed
    for run in 1 2 3; do
        start=${EPOCHREALTIME/,/.}
        "$@" > /dev/null
        end=${EPOCHREALTIME/,/.}
        elapsed=$(echo "$end - $start" | bc)
        if [ -z "$best" ] || [ "$(echo "$elapsed < $best" | bc)" = 1 ]; then
            best=$elapsed
        fi
    done
    printf '%.2f' "$best"
}
echo "parse: $(seconds "$command" parse --input-layer morphemes \
    --model "$work/cabocha-$posset.model" --input "$work/$name.morphemes")"
echo "cabocha: $(seconds cabocha 1 "$work/$name.morphemes")"

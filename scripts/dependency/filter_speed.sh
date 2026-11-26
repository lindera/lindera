#!/usr/bin/env bash
# What the token filter `japanese_dependency` costs: the tokens per second of
# `lindera tokenize` without and with the filter, and the time the filter
# adds to a token.
#
# Usage: filter_speed.sh [--dict <URI or path>] <model file> [<repetitions> [<data directory>]]
#   --dict            The dictionary of the model, when it is not IPADIC:
#                     the directory of a dictionary, such as the one
#                     `lindera download unidic-cwj` installs. The embedded
#                     IPADIC when it is left out.
#   <model file>      A model with a chunker and a parser for the
#                     dictionary, as `parser_experiments.sh final` leaves it
#                     in <data directory>/parser/full/default/model.
#   <repetitions>     How often the text is repeated (default 20).
#   <data directory>  Defaults to scripts/dependency/data and holds gsd/
#                     (fetch_corpora.sh).
#
# The text is the `# text = ` lines of the test file of UD Japanese-GSD, one
# sentence per line, which the command tokenizes line by line. The command is
# built with `embed-ipadic,dependency` and writes the MeCab format, which the
# filter does not change. It is run five times on an empty input and on the
# repeated text, without and with the filter; the fastest run of each is
# kept, and the time on the empty input is taken off. The difference of the
# two empty runs is the time to make the filter, which reads the model.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
dict=embedded://ipadic
if [ "${1:-}" = --dict ]; then
    dict=$2
    shift 2
    # A directory, not a URI: its full path.
    case "$dict" in
    *://*) ;;
    *) dict=$(readlink -f "$dict") ;;
    esac
fi
model=$(readlink -f "$1")
repetitions=${2:-20}
data=$(readlink -f "${3:-$here/data}")
work=$data/filter_speed
mkdir -p "$work"

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli \
    --features embed-ipadic,dependency
command=$root/target/release/lindera
filter="japanese_dependency:{\"model\":\"$model\"}"

sed -n 's/^# text = //p' "$data/gsd/ja_gsd-ud-test.conllu" > "$work/once.txt"
: > "$work/empty.txt"
: > "$work/long.txt"
for ((index = 0; index < repetitions; index++)); do
    cat "$work/once.txt" >> "$work/long.txt"
done

# The fastest of five runs, in seconds: the input, then the other arguments.
fastest() {
    local input=$1 best="" start end seconds
    shift
    for _ in 1 2 3 4 5; do
        start=${EPOCHREALTIME/,/.}
        "$command" tokenize --dict "$dict" "$@" "$input" > "$work/out.txt"
        end=${EPOCHREALTIME/,/.}
        seconds=$(echo "$end - $start" | bc -l)
        if [ -z "$best" ] || [ "$(echo "$seconds < $best" | bc -l)" = 1 ]; then
            best=$seconds
        fi
    done
    echo "$best"
}

sentences=$(wc -l < "$work/long.txt" | tr -d ' ')
load_plain=$(fastest "$work/empty.txt")
plain=$(fastest "$work/long.txt")
cp "$work/out.txt" "$work/plain.txt"
load_filter=$(fastest "$work/empty.txt" --token-filter "$filter")
with=$(fastest "$work/long.txt" --token-filter "$filter")
if ! cmp -s "$work/out.txt" "$work/plain.txt"; then
    echo "error: the filter changed the MeCab output" >&2
    exit 1
fi
tokens=$(grep -vc '^EOS$' "$work/out.txt" || true)

printf 'sentences: %d, tokens: %d\n' "$sentences" "$tokens"
printf 'making the filter: %.1f ms\n' "$(echo "($load_filter - $load_plain) * 1000" | bc -l)"
printf 'without the filter: %.0f tokens/s\n' "$(echo "$tokens / ($plain - $load_plain)" | bc -l)"
printf 'with the filter: %.0f tokens/s, %.0f sentences/s\n' \
    "$(echo "$tokens / ($with - $load_filter)" | bc -l)" \
    "$(echo "$sentences / ($with - $load_filter)" | bc -l)"
printf 'the filter adds: %.1f microseconds per token\n' \
    "$(echo "(($with - $load_filter) - ($plain - $load_plain)) / $tokens * 1000000" | bc -l)"

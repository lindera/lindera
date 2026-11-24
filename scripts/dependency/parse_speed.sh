#!/usr/bin/env bash
# The speed of `lindera-dependency parse` on plain text: the sentences per
# second of the command of a user, from the text to the lattice format.
#
# Usage: parse_speed.sh <model file> [<repetitions> [<data directory>]]
#   <model file>      A model with a chunker and a parser for IPADIC, as
#                     `parser_experiments.sh final` leaves it in
#                     <data directory>/parser/full/default/model.
#   <repetitions>     How often the text is repeated in the long input
#                     (default 100).
#   <data directory>  Defaults to scripts/dependency/data and holds gsd/
#                     (fetch_corpora.sh).
#
# The text is the `# text = ` lines of the test file of UD Japanese-GSD, one
# sentence per line. The command is built with `cli,embed-ipadic`, which is
# the build without the training code, and is run five times on an empty
# input, on the text once, and on the text repeated; the fastest run of
# each is kept. Loading the dictionary and the model is the time on the
# empty input, and the sentences per second without it come from the
# difference of the long run and the empty one.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
model=$(readlink -f "$1")
repetitions=${2:-100}
data=$(readlink -f "${3:-$here/data}")
work=$data/parse_speed
mkdir -p "$work"

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency \
    --features cli,embed-ipadic
command=$root/target/release/lindera-dependency

sed -n 's/^# text = //p' "$data/gsd/ja_gsd-ud-test.conllu" > "$work/once.txt"
: > "$work/empty.txt"
: > "$work/long.txt"
for ((index = 0; index < repetitions; index++)); do
    cat "$work/once.txt" >> "$work/long.txt"
done

# The fastest of five runs, in seconds.
fastest() {
    local best="" start end seconds
    for _ in 1 2 3 4 5; do
        start=${EPOCHREALTIME/,/.}
        "$command" parse --model "$model" --dict embedded://ipadic \
            --input "$1" --output "$work/out.lattice"
        end=${EPOCHREALTIME/,/.}
        seconds=$(echo "$end - $start" | bc -l)
        if [ -z "$best" ] || [ "$(echo "$seconds < $best" | bc -l)" = 1 ]; then
            best=$seconds
        fi
    done
    echo "$best"
}

sentences=$(wc -l < "$work/once.txt" | tr -d ' ')
total=$((sentences * repetitions))
load=$(fastest "$work/empty.txt")
once=$(fastest "$work/once.txt")
long=$(fastest "$work/long.txt")
written=$(grep -c '^EOS$' "$work/out.lattice" || true)
if [ "$written" != "$total" ]; then
    echo "error: $written sentences written for $total lines" >&2
    exit 1
fi

printf 'sentences: %d\n' "$sentences"
printf 'load (empty input): %.1f ms\n' "$(echo "$load * 1000" | bc -l)"
printf 'once, with the load: %.1f ms, %.0f sentences/s\n' \
    "$(echo "$once * 1000" | bc -l)" "$(echo "$sentences / $once" | bc -l)"
printf '%d times, with the load: %.2f s, %.0f sentences/s\n' \
    "$repetitions" "$long" "$(echo "$total / $long" | bc -l)"
printf '%d times, without the load: %.0f sentences/s\n' \
    "$repetitions" "$(echo "$total / ($long - $load)" | bc -l)"

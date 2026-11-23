#!/usr/bin/env bash
# Converts UD Japanese-GSD (train, dev, test) and UD Japanese-PUD to training
# data with `lindera-dependency convert` and the embedded IPADIC, and shows
# the report of each file: the sentences kept and dropped, the bunsetsu
# merged, and how often the head word of the schema is the content head of
# the corpus.
#
# Usage: convert_corpora.sh [<data directory>]
#   <data directory> defaults to scripts/dependency/data and holds gsd/ and
#   pud/ (fetch_corpora.sh). The lattice files and the reports go to
#   <data directory>/lattice.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
data=$(readlink -f "${1:-$here/data}")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features embed-ipadic
bin="$root/target/release/lindera-dependency"

mkdir -p "$data/lattice"
for entry in gsd/ja_gsd-ud-train gsd/ja_gsd-ud-dev gsd/ja_gsd-ud-test pud/ja_pud-ud-test; do
    name=$(basename "$entry")
    # From the directory of the corpus, so that the report names the file
    # without a path.
    env -C "$data/$(dirname "$entry")" "$bin" convert --dict embedded://ipadic \
        --input "$name.conllu" --output "$data/lattice/$name.lattice" \
        2> "$data/lattice/$name.report"
    cat "$data/lattice/$name.report"
    echo
done

#!/usr/bin/env bash
# Converts UD Japanese-GSD (train, dev, test) and UD Japanese-PUD to training
# data with `lindera dependency convert` and a dictionary, and shows the
# report of each file: the sentences kept and dropped, the bunsetsu merged,
# and how often the head word of the schema is the content head of the
# corpus.
#
# Usage: convert_corpora.sh [--dictionary <name>] [--dict <URI or path>] [<data directory>]
#   --dictionary  ipadic (the default), unidic, unidic-cwj or unidic-csj.
#   --dict        The dictionary to tokenize with, when it is not the one
#                 this script gets: the embedded IPADIC for ipadic, and for
#                 the others the dictionary of the release of the version of
#                 the workspace, which `lindera download` installs into
#                 <data directory>/dictionaries. Nothing is embedded for
#                 UniDic: embedding UniDic CWJ or CSJ takes 8 to 11 GiB of
#                 memory to compile.
#   <data directory> defaults to scripts/dependency/data and holds gsd/ and
#   pud/ (fetch_corpora.sh). The lattice files and the reports go to
#   <data directory>/lattice for ipadic, and to
#   <data directory>/<name>/lattice for another dictionary.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
# shellcheck source=scripts/dependency/dictionary.sh
. "$here/dictionary.sh"
parse_dictionary_arguments "$@"
data=$(readlink -f "${rest[0]:-$here/data}")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli --features embed-ipadic
bin="$root/target/release/lindera"
resolve_dictionary "$root" "$data"

output=$(dictionary_directory "$data")/lattice
mkdir -p "$output"
for entry in gsd/ja_gsd-ud-train gsd/ja_gsd-ud-dev gsd/ja_gsd-ud-test pud/ja_pud-ud-test; do
    name=$(basename "$entry")
    # From the directory of the corpus, so that the report names the file
    # without a path.
    env -C "$data/$(dirname "$entry")" "$bin" dependency convert --dict "$dict" \
        --input "$name.conllu" --output "$output/$name.lattice" \
        2> "$output/$name.report"
    cat "$output/$name.report"
    echo
done

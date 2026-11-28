#!/usr/bin/env bash
# Writes the training instances of the parser for UD Japanese-GSD (train,
# dev, test) and UD Japanese-PUD with `lindera dependency instances`, and
# shows the report of each file: the sentences, the bunsetsu and the
# instances. The command stops at a sentence whose tree does not come back
# when its answer decides, so a report for every file says that every tree
# did.
#
# Usage: parser_instances.sh [--dictionary <name>] [<data directory>]
#   --dictionary  ipadic (the default), unidic, unidic-cwj or unidic-csj:
#                 the dictionary convert_corpora.sh was run with, whose
#                 schema makes the features.
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   for ipadic and <name>/lattice/ for another dictionary
#   (convert_corpora.sh). The instances and the reports go to instances/
#   next to it.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
# shellcheck source=scripts/dependency/dictionary.sh
. "$here/dictionary.sh"
parse_dictionary_arguments "$@"
data=$(readlink -f "${rest[0]:-$here/data}")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-cli --features embed-ipadic
bin="$root/target/release/lindera"

directory=$(dictionary_directory "$data")
mkdir -p "$directory/instances"
for name in ja_gsd-ud-train ja_gsd-ud-dev ja_gsd-ud-test ja_pud-ud-test; do
    # From the directory of the lattice files, so that the report names the
    # file without a path.
    env -C "$directory/lattice" "$bin" dependency instances --dictionary-name "$dictionary" \
        --input "$name.lattice" \
        --output "$directory/instances/$name.instances" \
        2> "$directory/instances/$name.report"
    cat "$directory/instances/$name.report"
    echo
done

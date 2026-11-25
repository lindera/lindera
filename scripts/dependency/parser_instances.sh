#!/usr/bin/env bash
# Writes the training instances of the parser for UD Japanese-GSD (train,
# dev, test) and UD Japanese-PUD with `lindera-dependency instances`, and
# shows the report of each file: the sentences, the bunsetsu and the
# instances. The command stops at a sentence whose tree does not come back
# when its answer decides, so a report for every file says that every tree
# did.
#
# Usage: parser_instances.sh [<data directory>]
#   <data directory> defaults to scripts/dependency/data and holds lattice/
#   (convert_corpora.sh). The instances and the reports go to
#   <data directory>/instances.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")
data=$(readlink -f "${1:-$here/data}")

cargo build --manifest-path "$root/Cargo.toml" --release -p lindera-dependency --features train,embed-ipadic
bin="$root/target/release/lindera-dependency"

mkdir -p "$data/instances"
for name in ja_gsd-ud-train ja_gsd-ud-dev ja_gsd-ud-test ja_pud-ud-test; do
    # From the directory of the lattice files, so that the report names the
    # file without a path.
    env -C "$data/lattice" "$bin" instances --input "$name.lattice" \
        --output "$data/instances/$name.instances" \
        2> "$data/instances/$name.report"
    cat "$data/instances/$name.report"
    echo
done

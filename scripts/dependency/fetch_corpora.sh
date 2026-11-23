#!/usr/bin/env bash
# Gets UD Japanese-GSD and UD Japanese-PUD at the commits the numbers of the
# design notes were taken at.
#
# The corpora are not part of this repository: GSD is under CC BY-SA 4.0 and
# PUD under CC BY-SA 3.0. See the README of lindera-dependency.
#
# Usage: fetch_corpora.sh [<data directory>]
#   <data directory> defaults to scripts/dependency/data, which git ignores.
#   The corpora go to <data directory>/gsd and <data directory>/pud.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
data=${1:-$here/data}

fetch() {
    local directory=$1 repository=$2 commit=$3
    if [ -d "$directory/.git" ]; then
        echo "$directory: already there"
    else
        mkdir -p "$directory"
        git -C "$directory" init --quiet
        git -C "$directory" remote add origin "https://github.com/UniversalDependencies/$repository.git"
        git -C "$directory" fetch --quiet --depth 1 origin "$commit"
        git -C "$directory" checkout --quiet FETCH_HEAD
    fi
    echo "$directory: $(git -C "$directory" rev-parse HEAD)"
}

fetch "$data/gsd" UD_Japanese-GSD c662dc96ed5ee5ac0fc8b5e44261557e431f50e6
fetch "$data/pud" UD_Japanese-PUD cf69c5c100e8dfd2439f6e97924188db83ae39d7

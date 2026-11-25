#!/usr/bin/env bash
# Builds the model of lindera-dependency for IPADIC as it is released: from
# the corpus at its fixed commit and the sources of this repository to the
# model file, its notice and their checksums.
#
# The steps: get UD Japanese-GSD (fetch_corpora.sh), build the command with
# the toolchain named below and the dependencies of Cargo.lock, convert the
# train file with the embedded IPADIC, train the chunker and the parser with
# the defaults of the command, and write
#
#   <output directory>/lindera-dependency-ipadic-<version>.model
#   <output directory>/NOTICE.txt
#   <output directory>/SHA256SUMS
#
# Only the train file is used: dev, test and UD Japanese-PUD stay unseen, so
# that the accuracy of the file can be measured on them.
#
# Usage: build_model.sh [--version <version>] [--toolchain <toolchain>]
#                       [--check <sha256>] [--allow-dirty] [<output directory>]
#   --version      The version of the model, in the file name and the
#                  notice. Defaults to 1.0.0.
#   --toolchain    The Rust toolchain (rustup) the command is built with.
#                  Defaults to 1.100.0, with which the released file was built.
#   --check        The SHA-256 the model file has to have. The script fails
#                  when the file it built has another one.
#   --allow-dirty  Build from a tree with uncommitted changes. The notice
#                  then says so; such a file is not for a release.
#   <output directory> defaults to scripts/dependency/data/model, which git
#   ignores. It has to be empty or not to exist: the corpus is fetched into
#   it, so that nothing of an earlier run is used.
#
# The same sources, corpus, toolchain and platform give the same bytes. See
# the README of this directory for what is known about other platforms.
set -euo pipefail

here=$(dirname "$(readlink -f "$0")")
root=$(readlink -f "$here/../..")

version=1.0.0
toolchain=1.100.0
check=
allow_dirty=false
out=$here/data/model
while [ $# -gt 0 ]; do
    case "$1" in
    --version)
        version=$2
        shift 2
        ;;
    --toolchain)
        toolchain=$2
        shift 2
        ;;
    --check)
        check=$2
        shift 2
        ;;
    --allow-dirty)
        allow_dirty=true
        shift
        ;;
    -*)
        echo "unknown option $1" >&2
        exit 2
        ;;
    *)
        out=$1
        shift
        ;;
    esac
done

# sha256 <file>: the SHA-256 of a file, in hexadecimal.
sha256() {
    if command -v sha256sum > /dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# The sources: a commit, and nothing on top of it.
commit=$(git -C "$root" rev-parse HEAD)
if [ -n "$(git -C "$root" status --porcelain)" ]; then
    if [ "$allow_dirty" != true ]; then
        echo "the tree of $root has uncommitted changes; commit them, or pass --allow-dirty" >&2
        exit 1
    fi
    commit="$commit, with uncommitted changes"
fi

if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then
    echo "$out is not empty; give a directory that is empty or does not exist" >&2
    exit 1
fi
mkdir -p "$out"
out=$(readlink -f "$out")
work=$out/work
mkdir -p "$work"

# The corpus, at the commit fetch_corpora.sh names.
"$here/fetch_corpora.sh" "$work/corpora"
corpus=$(git -C "$work/corpora/gsd" rev-parse HEAD)

# The command. --locked: the dependencies of Cargo.lock, and no others.
cargo "+$toolchain" build --manifest-path "$root/Cargo.toml" --locked --release \
    -p lindera-dependency --features train,embed-ipadic
bin="$root/target/release/lindera-dependency"
rustc=$(rustc "+$toolchain" --version)
host=$(rustc "+$toolchain" -vV | sed -n 's/^host: //p')
libc=$(getconf GNU_LIBC_VERSION 2> /dev/null || uname -sr)

# The training data: the train file only. From the directory of the corpus,
# so that the report names the file without a path.
env -C "$work/corpora/gsd" "$bin" convert --dict embedded://ipadic \
    --input ja_gsd-ud-train.conllu --output "$work/train.lattice" \
    2> "$work/train.report"
cat "$work/train.report"

# The chunker, then the parser with the chunker in its file. No option: the
# defaults of the command are the settings of the released model. The number
# of threads of train-chunker changes how long the training takes and not
# its result. The line of each iteration stays in the log.
"$bin" train-chunker --input "$work/train.lattice" --output "$work/chunker.model" \
    2> "$work/train-chunker.log" || {
    cat "$work/train-chunker.log" >&2
    exit 1
}
grep -v '^iter: ' "$work/train-chunker.log"
model=$out/lindera-dependency-ipadic-$version.model
"$bin" train-parser --input "$work/train.lattice" \
    --chunker "$work/chunker.model" --output "$model"

# The notice: the template, with what this run used, the license of the
# repository and the notice of the dictionary.
sed -e "s|@VERSION@|$version|g" \
    -e "s|@LINDERA_COMMIT@|$commit|g" \
    -e "s|@CORPUS_COMMIT@|$corpus|g" \
    -e "s|@RUSTC@|$rustc|g" \
    -e "s|@PLATFORM@|$host, $libc|g" \
    "$here/model_notice_ipadic.txt" |
    while IFS= read -r line; do
        case "$line" in
        @LICENSE@) cat "$root/LICENSE" ;;
        @IPADIC_NOTICE@) cat "$root/lindera-ipadic/NOTICE.txt" ;;
        *) printf '%s\n' "$line" ;;
        esac
    done > "$out/NOTICE.txt"

digest=$(sha256 "$model")
{
    echo "$digest  $(basename "$model")"
    echo "$(sha256 "$out/NOTICE.txt")  NOTICE.txt"
} > "$out/SHA256SUMS"

echo
echo "lindera: $commit"
echo "corpus: UD Japanese-GSD $corpus"
echo "toolchain: $rustc"
echo "platform: $host, $libc"
echo "training data: $(sha256 "$work/train.lattice")  train.lattice"
echo "chunker: $(sha256 "$work/chunker.model")  chunker.model"
echo "model: $digest  $(basename "$model") ($(wc -c < "$model" | tr -d ' ') bytes)"

if [ -n "$check" ]; then
    if [ "$digest" = "$check" ]; then
        echo "check: the model has the SHA-256 given"
    else
        echo "check: the model does NOT have the SHA-256 given ($check)" >&2
        exit 1
    fi
fi

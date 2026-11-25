#!/usr/bin/env bash
# Builds the model of lindera-dependency for IPADIC as a release of Lindera
# publishes it: from the corpus at its fixed commit and the sources of this
# repository to the archive that goes on the page of the release.
#
# The steps: get UD Japanese-GSD (fetch_corpora.sh), build the command with
# the dependencies of Cargo.lock, convert the train file with the embedded
# IPADIC, train the chunker and the parser with the defaults of the command,
# and write
#
#   <output directory>/lindera-dependency-ipadic/dependency.model
#   <output directory>/lindera-dependency-ipadic/model.json
#   <output directory>/lindera-dependency-ipadic/NOTICE.txt
#   <output directory>/lindera-dependency-ipadic-<version>.zip
#
# The archive holds the directory. `model.json` names the dictionary and the
# version of Lindera, as the `metadata.json` of a dictionary does.
#
# Only the train file is used: dev, test and UD Japanese-PUD stay unseen, so
# that the accuracy of the file can be measured on them.
#
# A build from the repository embeds the model it is given, not one it
# downloads:
#
#   LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE=<output directory>/lindera-dependency-ipadic/dependency.model
#
# Usage: build_model.sh [--version <version>] [--toolchain <toolchain>]
#                       [--check <sha256>] [--allow-dirty] [<output directory>]
#   --version      The version of Lindera the model is released with, in the
#                  name of the archive, model.json and the notice. Defaults
#                  to the version of the workspace.
#   --toolchain    The Rust toolchain (rustup) the command is built with,
#                  such as 1.100.0. Defaults to the active one.
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

version=
toolchain=
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

# The version of the workspace: the first `version` of the root manifest,
# which is the one of [workspace.package].
if [ -z "$version" ]; then
    version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)
    if [ -z "$version" ]; then
        echo "cannot read the version of the workspace from $root/Cargo.toml" >&2
        exit 1
    fi
fi

# With a toolchain named, cargo and rustc are run as `cargo +<toolchain>`.
cargo=(cargo)
rustc=(rustc)
if [ -n "$toolchain" ]; then
    cargo+=("+$toolchain")
    rustc+=("+$toolchain")
fi

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
key=lindera-dependency-ipadic
mkdir -p "$work" "$out/$key"

# The corpus, at the commit fetch_corpora.sh names.
"$here/fetch_corpora.sh" "$work/corpora"
corpus=$(git -C "$work/corpora/gsd" rev-parse HEAD)

# The command. --locked: the dependencies of Cargo.lock, and no others.
"${cargo[@]}" build --manifest-path "$root/Cargo.toml" --locked --release \
    -p lindera-dependency --features train,embed-ipadic
bin="$root/target/release/lindera-dependency"
compiler=$("${rustc[@]}" --version)
host=$("${rustc[@]}" -vV | sed -n 's/^host: //p')
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
model=$out/$key/dependency.model
"$bin" train-parser --input "$work/train.lattice" \
    --chunker "$work/chunker.model" --output "$model"

# What the file is, for `lindera download` and `lindera list`.
cat > "$out/$key/model.json" << EOF
{
  "name": "dependency-ipadic",
  "dictionary": "ipadic",
  "lindera_version": "$version"
}
EOF

# The notice: the template, with what this run used, the license of the
# repository and the notice of the dictionary.
sed -e "s|@VERSION@|$version|g" \
    -e "s|@LINDERA_COMMIT@|$commit|g" \
    -e "s|@CORPUS_COMMIT@|$corpus|g" \
    -e "s|@RUSTC@|$compiler|g" \
    -e "s|@PLATFORM@|$host, $libc|g" \
    "$here/model_notice_ipadic.txt" |
    while IFS= read -r line; do
        case "$line" in
        @LICENSE@) cat "$root/LICENSE" ;;
        @IPADIC_NOTICE@) cat "$root/lindera-ipadic/NOTICE.txt" ;;
        *) printf '%s\n' "$line" ;;
        esac
    done > "$out/$key/NOTICE.txt"

# The archive of the release: one directory, as the archive of a dictionary.
# The times of the files are set and the extra fields left out, so that the
# same files give the same archive.
archive=$out/$key-$version.zip
TZ=UTC touch -t 198001010000 "$out/$key"/*
env -C "$out" TZ=UTC zip -q -X "$archive" \
    "$key/NOTICE.txt" "$key/dependency.model" "$key/model.json"

digest=$(sha256 "$model")
echo
echo "lindera: $commit"
echo "corpus: UD Japanese-GSD $corpus"
echo "toolchain: $compiler"
echo "platform: $host, $libc"
echo "training data: $(sha256 "$work/train.lattice")  train.lattice"
echo "chunker: $(sha256 "$work/chunker.model")  chunker.model"
echo "model: $digest  $key/dependency.model ($(wc -c < "$model" | tr -d ' ') bytes)"
echo "archive: $(sha256 "$archive")  $(basename "$archive")"

if [ -n "$check" ]; then
    if [ "$digest" = "$check" ]; then
        echo "check: the model has the SHA-256 given"
    else
        echo "check: the model does NOT have the SHA-256 given ($check)" >&2
        exit 1
    fi
fi

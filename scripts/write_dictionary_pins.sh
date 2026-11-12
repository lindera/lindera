#!/usr/bin/env bash
#
# Writes the prebuilt.json of each official dictionary crate, pinning the
# archive that a release has put on the page of its version (#1362).
#
# A release builds the dictionaries from source and uploads them as
# `lindera-<name>-<version>.zip` next to the binaries. Just before it publishes
# the dictionary crates, release.yml downloads those archives and runs this
# script, so that each published crate pins its archive by SHA-256 and an
# `embed-*` build of it downloads the archive instead of building the
# dictionary. The files are not committed: a build from the repository has no
# prebuilt.json and builds from source.
#
# Usage:
#   scripts/write_dictionary_pins.sh <tag> <version> <archive dir> <crate>...
#
# Example:
#   scripts/write_dictionary_pins.sh v7.0.0 7.0.0 dictionary-archives \
#     lindera-ipadic lindera-unidic
#
# Each <archive dir>/<crate>-<version>.zip must hold the directory <crate>/
# with the dictionary's metadata.json. Prints one line per crate, and fails
# without writing anything for a crate whose archive is missing or holds no
# such directory.

set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <tag> <version> <archive dir> <crate>..." >&2
  exit 1
}

[ "$#" -ge 4 ] || usage
tag=$1
version=$2
archive_dir=$3
shift 3

base_url="https://github.com/lindera/lindera/releases/download"

# The format version of the dictionaries this checkout builds, which the
# crates it publishes read.
format_version=$(sed -n 's/^pub const DICTIONARY_FORMAT_VERSION: u32 = \([0-9]*\);$/\1/p' \
  lindera-dictionary/src/dictionary/metadata.rs)
if [ -z "$format_version" ]; then
  echo "error: could not read DICTIONARY_FORMAT_VERSION" >&2
  exit 1
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# Check every archive before writing any pin, so that a failure leaves no crate
# half pinned.
for crate in "$@"; do
  archive="$archive_dir/$crate-$version.zip"
  if [ ! -f "$archive" ]; then
    echo "error: no archive $archive for $crate" >&2
    exit 1
  fi
  if ! unzip -Z1 "$archive" | grep -Fqx "$crate/metadata.json"; then
    echo "error: $archive holds no $crate/metadata.json" >&2
    exit 1
  fi
  if [ ! -f "$crate/build.rs" ]; then
    echo "error: $crate is not a crate of this checkout" >&2
    exit 1
  fi
done

for crate in "$@"; do
  archive="$archive_dir/$crate-$version.zip"
  sha256=$(sha256_of "$archive")
  cat >"$crate/prebuilt.json" <<EOF
{
  "key": "$crate",
  "format_version": $format_version,
  "base_url": "$base_url",
  "path": "$tag/$crate-$version.zip",
  "sha256": "$sha256"
}
EOF
  echo "$crate: $base_url/$tag/$crate-$version.zip $sha256"
done

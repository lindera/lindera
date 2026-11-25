#!/usr/bin/env bash
#
# Writes the prebuilt.json of lindera-dependency, pinning the archive of the
# model for IPADIC that a release has put on the page of its version (#1440).
#
# A release builds the model with scripts/dependency/build_model.sh and
# uploads it as `lindera-dependency-ipadic-<version>.zip` next to the
# dictionaries. Just before it publishes the crates, release.yml downloads
# that archive and runs this script, so that the published crate pins the
# archive by SHA-256 and a build of it with `embed-model-ipadic` downloads the
# archive. The file is not committed: a build from the repository has no
# prebuilt.json and embeds the file that
# LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE names.
#
# Usage:
#   scripts/write_dependency_model_pin.sh <tag> <version> <archive dir>
#
# Example:
#   scripts/write_dependency_model_pin.sh v7.1.0 7.1.0 model-archives
#
# <archive dir>/lindera-dependency-ipadic-<version>.zip must hold the
# directory lindera-dependency-ipadic/ with dependency.model, a model.json
# that names <version> as its lindera_version, and NOTICE.txt. Prints one
# line, and fails without writing anything for an archive that is missing or
# holds something else.

set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <tag> <version> <archive dir>" >&2
  exit 1
}

[ "$#" -eq 3 ] || usage
tag=$1
version=$2
archive_dir=$3

base_url="https://github.com/lindera/lindera/releases/download"
key=lindera-dependency-ipadic
crate=lindera-dependency
archive="$archive_dir/$key-$version.zip"

# The format version of the model files this checkout reads and writes,
# which the build script of the crate asks the pin for.
format_version=$(sed -n 's/^pub(crate) const VERSION: u32 = \([0-9]*\);$/\1/p' \
  "$crate/src/model.rs")
if [ -z "$format_version" ]; then
  echo "error: could not read the format version of a model file" >&2
  exit 1
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

if [ ! -f "$archive" ]; then
  echo "error: no archive $archive" >&2
  exit 1
fi
for file in dependency.model model.json NOTICE.txt; do
  if ! unzip -Z1 "$archive" | grep -Fqx "$key/$file"; then
    echo "error: $archive holds no $key/$file" >&2
    exit 1
  fi
done
if ! unzip -p "$archive" "$key/model.json" | grep -Fq "\"lindera_version\": \"$version\""; then
  echo "error: the model.json of $archive does not name the version $version" >&2
  exit 1
fi
if [ ! -f "$crate/build.rs" ]; then
  echo "error: $crate is not a crate of this checkout" >&2
  exit 1
fi

sha256=$(sha256_of "$archive")
cat >"$crate/prebuilt.json" <<EOF
{
  "key": "$key",
  "format_version": $format_version,
  "base_url": "$base_url",
  "path": "$tag/$key-$version.zip",
  "sha256": "$sha256"
}
EOF
echo "$crate: $base_url/$tag/$key-$version.zip $sha256"

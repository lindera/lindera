#!/usr/bin/env bash
#
# Writes the pins of lindera-dependency, one for each model it can embed:
# prebuilt-<dictionary>.json pins the archive of the model for that
# dictionary that a release has put on the page of its version (#1440,
# #1441).
#
# A release builds each model with scripts/dependency/build_model.sh and
# uploads it as `lindera-dependency-<dictionary>-<version>.zip` next to the
# dictionaries. Just before it publishes the crates, release.yml downloads
# those archives and runs this script, so that the published crate pins each
# archive by SHA-256 and a build of it with `embed-model-<dictionary>`
# downloads the archive. The files are not committed: a build from the
# repository has no pin and embeds the file that the variable of the model
# names, LINDERA_BUILD_DEPENDENCY_MODEL_IPADIC_FILE for IPADIC.
#
# Usage:
#   scripts/write_dependency_model_pin.sh <tag> <version> <archive dir> [<dictionary>...]
#
# Example:
#   scripts/write_dependency_model_pin.sh v7.1.0 7.1.0 model-archives
#
# The dictionaries default to all the crate has a model for: ipadic, unidic,
# unidic-cwj and unidic-csj. For each,
# <archive dir>/lindera-dependency-<dictionary>-<version>.zip must hold the
# directory lindera-dependency-<dictionary>/ with dependency.model, a
# model.json that names the dictionary and <version> as its lindera_version,
# and NOTICE.txt. Prints one line per model. Every archive is checked before
# any pin is written, so that an archive that is missing or holds something
# else leaves no pin.

set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <tag> <version> <archive dir> [<dictionary>...]" >&2
  exit 1
}

[ "$#" -ge 3 ] || usage
tag=$1
version=$2
archive_dir=$3
shift 3
if [ "$#" -gt 0 ]; then
  dictionaries=("$@")
else
  dictionaries=(ipadic unidic unidic-cwj unidic-csj)
fi

base_url="https://github.com/lindera/lindera/releases/download"
crate=lindera-dependency

# The format version of the model files this checkout reads and writes,
# which the build script of the crate asks the pin for.
format_version=$(sed -n 's/^pub(crate) const VERSION: u32 = \([0-9]*\);$/\1/p' \
  "$crate/src/model.rs")
if [ -z "$format_version" ]; then
  echo "error: could not read the format version of a model file" >&2
  exit 1
fi
if [ ! -f "$crate/build.rs" ]; then
  echo "error: $crate is not a crate of this checkout" >&2
  exit 1
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

for dictionary in "${dictionaries[@]}"; do
  key=lindera-dependency-$dictionary
  archive="$archive_dir/$key-$version.zip"
  # The build script has to read the pin this script is about to write.
  if ! grep -Fq "\"prebuilt-$dictionary.json\"" "$crate/build.rs"; then
    echo "error: $crate embeds no model for $dictionary" >&2
    exit 1
  fi
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
  metadata=$(unzip -p "$archive" "$key/model.json")
  if ! grep -Fq "\"lindera_version\": \"$version\"" <<<"$metadata"; then
    echo "error: the model.json of $archive does not name the version $version" >&2
    exit 1
  fi
  if ! grep -Fq "\"dictionary\": \"$dictionary\"" <<<"$metadata"; then
    echo "error: the model.json of $archive does not name the dictionary $dictionary" >&2
    exit 1
  fi
done

for dictionary in "${dictionaries[@]}"; do
  key=lindera-dependency-$dictionary
  archive="$archive_dir/$key-$version.zip"
  sha256=$(sha256_of "$archive")
  cat >"$crate/prebuilt-$dictionary.json" <<EOF
{
  "key": "$key",
  "format_version": $format_version,
  "base_url": "$base_url",
  "path": "$tag/$key-$version.zip",
  "sha256": "$sha256"
}
EOF
  echo "$crate: $base_url/$tag/$key-$version.zip $sha256"
done

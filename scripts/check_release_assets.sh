#!/usr/bin/env bash
#
# Checks that a directory holds the assets of a release, all of them and
# nothing else (#1413).
#
# release.yml builds the assets in separate jobs and gathers them in the
# `release-assets` job, which uploads them to a draft release in one go. This
# script runs there before the upload, and on a dry run (workflow_dispatch) as
# well, so that a build that stops producing its asset, or produces another
# name, fails before the release exists.
#
# Usage:
#   scripts/check_release_assets.sh <dir> <version>
#
# Example:
#   scripts/check_release_assets.sh release-assets 7.0.0
#
# Expects, directly in <dir>:
#   - lindera-<dictionary>-<version>.zip for each official dictionary
#   - lindera-dependency-ipadic-<version>.zip, the model of lindera-dependency
#   - lindera-<target>-<version>.zip for each target of the CLI
#   - six wheels, lindera-<version>-*.whl
#   - the sdist, lindera-<version>.tar.gz
#   - thirty PIE binaries, php_lindera-v<version>_php*.zip (the tag as PIE looks
#     it up; php_lindera-v<version>-dev_php*.zip on a dry run)
#   - the gem, lindera-<version>.gem
#
# Prints one line per kind. The lists and counts follow the matrices of
# build-dictionary, build-dependency-model, build, build-python-wheels and build-php-binaries in
# release.yml: update them together.

set -euo pipefail

usage() {
  echo "Usage: $(basename "$0") <dir> <version>" >&2
  exit 1
}

if [ "$#" -ne 2 ]; then
  usage
fi

dir=$1
version=$2

if [ ! -d "$dir" ]; then
  echo "error: $dir is not a directory" >&2
  exit 1
fi

dictionaries="ipadic ipadic-neologd unidic unidic-cwj unidic-csj sudachidict ko-dic cc-cedict jieba"
targets="x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-unknown-linux-musl aarch64-unknown-linux-musl aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc aarch64-pc-windows-msvc"
wheels=6
pie_binaries=30

failed=0
expected=0

# Fails the check, after every kind has been reported.
fail() {
  echo "error: $1" >&2
  failed=1
}

# Checks that each of the named files exists.
check_files() {
  local kind=$1
  shift
  local found=0
  local name
  for name in "$@"; do
    expected=$((expected + 1))
    if [ -f "$dir/$name" ]; then
      found=$((found + 1))
    else
      fail "$kind: $name is missing"
    fi
  done
  echo "$kind: $found of $#"
}

# Checks that the files matching a pattern are as many as expected.
check_count() {
  local kind=$1
  local pattern=$2
  local want=$3
  local found
  found=$(find "$dir" -maxdepth 1 -type f -name "$pattern" | wc -l | tr -d ' ')
  expected=$((expected + want))
  if [ "$found" -ne "$want" ]; then
    fail "$kind: expected $want files matching $pattern, found $found"
  fi
  echo "$kind: $found of $want"
}

names=()
for dictionary in $dictionaries; do
  names+=("lindera-$dictionary-$version.zip")
done
check_files "dictionaries" "${names[@]}"
check_files "dependency model" "lindera-dependency-ipadic-$version.zip"

names=()
for target in $targets; do
  names+=("lindera-$target-$version.zip")
done
check_files "CLI archives" "${names[@]}"

check_count "wheels" "lindera-$version-*.whl" "$wheels"
check_files "sdist" "lindera-$version.tar.gz"
check_count "PIE binaries" "php_lindera-v${version}*_php*.zip" "$pie_binaries"
check_files "gem" "lindera-$version.gem"

# Anything else would be uploaded to the release with the assets.
total=$(find "$dir" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')
if [ "$total" -ne "$expected" ]; then
  fail "$dir holds $total entries, expected $expected"
  find "$dir" -mindepth 1 -maxdepth 1 | sort >&2
fi
echo "total: $total of $expected"

exit "$failed"

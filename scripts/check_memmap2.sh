#!/usr/bin/env bash
#
# Checks that `memmap2` is linked only into the builds that turn the `mmap`
# feature on (#1414).
#
# The workspace depends on lindera-dictionary without its default features,
# so that `mmap`, and with it `memmap2`, comes only from a crate that asks
# for it: the `mmap` feature of lindera-segmenter, which lindera and
# lindera-analysis forward and turn on by default. A dependency on
# lindera-dictionary that keeps its default features would bring `memmap2`
# back into every build of the crate that has it, whatever its features, as
# before #1414.
#
# Usage:
#   scripts/check_memmap2.sh
#
# Runs `cargo tree --locked -i memmap2` for each build below, from the
# workspace root, and prints one line per build. Fails if a build has
# `memmap2` that should not, lacks it when it should have it, or if
# `cargo tree` fails for another reason.

set -euo pipefail

failed=0

# Prints "absent", "present" or "error" for the dependency graph of the
# `cargo tree` arguments given, and the output of `cargo tree` on stderr when
# it fails for another reason than a graph without `memmap2`.
memmap2_in() {
  local out
  if out=$(cargo tree --locked "$@" -i memmap2 2>&1); then
    if grep -q '^memmap2 v' <<<"$out"; then
      echo present
    else
      echo error
      echo "$out" >&2
    fi
  elif grep -q 'did not match any packages' <<<"$out"; then
    echo absent
  else
    echo error
    echo "$out" >&2
  fi
}

# Checks one build: <expected state> <cargo tree arguments>...
expect() {
  local expected=$1
  shift
  local actual
  actual=$(memmap2_in "$@")
  if [ "$actual" = "$expected" ]; then
    echo "ok: memmap2 $actual: $*"
  else
    echo "FAILED: memmap2 $actual, expected $expected: $*"
    failed=1
  fi
}

# Without the `mmap` feature, also through the build scripts of an embedded
# dictionary and through the analysis and training features.
expect absent -p lindera --no-default-features
expect absent -p lindera --no-default-features --features embed-ipadic
expect absent -p lindera --no-default-features --features analysis
expect absent -p lindera --no-default-features --features train
expect absent -p lindera-segmenter --no-default-features
expect absent -p lindera-trainer

# With the `mmap` feature, on by default or named.
expect present -p lindera
expect present -p lindera --no-default-features --features mmap
expect present -p lindera-segmenter

exit "$failed"

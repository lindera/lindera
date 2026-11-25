#!/usr/bin/env bash
#
# Checks that lindera-dependency stays a core that needs no tokenizer and no
# dictionary (#1418).
#
# The crate takes any sequence of morphemes, so that it can be used with
# another tokenizer and in a browser. It must therefore build for
# wasm32-unknown-unknown, and its default build must not depend on
# lindera-segmenter, lindera-analysis, the lindera facade, lindera-dictionary
# or a dictionary crate. lindera-crf, the CRF engine, is the one crate of the
# workspace it may depend on.
#
# The `cli` feature is the command for a user (#1439): it tokenizes with
# lindera-segmenter, and must not bring the training code, which is
# lindera-crf.
#
# The token filter japanese_dependency (#1438) makes lindera-analysis depend
# on this crate behind its `dependency` feature, which lindera,
# lindera-cli, lindera-binding and the five bindings forward. The builds
# without the feature must not link the crate, and the build with it has
# to.
#
# Usage:
#   scripts/check_dependency_crate.sh
#
# Builds the crate for wasm32-unknown-unknown and reads
# `cargo tree --locked -p lindera-dependency -e normal`, from the workspace
# root, for the default build and for the build with `cli`. Prints one line
# per check, and fails if the build fails, if the default build depends on
# another crate of the workspace than lindera-crf, or if the build with
# `cli` depends on lindera-crf or does not depend on lindera-segmenter.
# Then reads the tree of lindera-analysis, lindera, lindera-cli,
# lindera-binding and the five bindings without and with `dependency`, and
# fails if lindera-dependency is in the first or is missing from the
# second.

set -euo pipefail

failed=0

if cargo build --locked -p lindera-dependency --target wasm32-unknown-unknown; then
  echo "ok: lindera-dependency builds for wasm32-unknown-unknown"
else
  echo "error: lindera-dependency does not build for wasm32-unknown-unknown" >&2
  failed=1
fi

if ! tree=$(cargo tree --locked -p lindera-dependency -e normal --prefix none); then
  echo "error: cargo tree failed" >&2
  exit 1
fi
# One line per crate: `<name> v<version> ...`.
forbidden=$(echo "$tree" | awk '$1 ~ /^lindera/ && $1 != "lindera-dependency" && $1 != "lindera-crf" { print $1 }' | sort -u)
if [ -n "$forbidden" ]; then
  echo "error: lindera-dependency depends on:" >&2
  echo "$forbidden" >&2
  failed=1
else
  echo "ok: lindera-dependency depends on no tokenizer or dictionary crate"
fi

if ! tree=$(cargo tree --locked -p lindera-dependency --features cli -e normal --prefix none); then
  echo "error: cargo tree --features cli failed" >&2
  exit 1
fi
if echo "$tree" | awk '$1 == "lindera-crf" { found = 1 } END { exit !found }'; then
  echo "error: lindera-dependency with cli depends on lindera-crf, the training code" >&2
  failed=1
else
  echo "ok: lindera-dependency with cli has no training code"
fi
# The check above passes on an empty tree: the tokenizer has to be there.
if echo "$tree" | awk '$1 == "lindera-segmenter" { found = 1 } END { exit !found }'; then
  echo "ok: lindera-dependency with cli depends on lindera-segmenter"
else
  echo "error: lindera-dependency with cli does not depend on lindera-segmenter" >&2
  failed=1
fi

for crate in lindera-analysis lindera lindera-cli lindera-binding \
  lindera-python lindera-nodejs lindera-ruby lindera-php lindera-wasm; do
  if ! tree=$(cargo tree --locked -p "$crate" -e normal --prefix none); then
    echo "error: cargo tree -p $crate failed" >&2
    exit 1
  fi
  if echo "$tree" | awk '$1 == "lindera-dependency" { found = 1 } END { exit !found }'; then
    echo "error: $crate depends on lindera-dependency without the dependency feature" >&2
    failed=1
  else
    echo "ok: $crate without the dependency feature does not depend on lindera-dependency"
  fi
  if ! tree=$(cargo tree --locked -p "$crate" --features dependency -e normal --prefix none); then
    echo "error: cargo tree -p $crate --features dependency failed" >&2
    exit 1
  fi
  if echo "$tree" | awk '$1 == "lindera-dependency" { found = 1 } END { exit !found }'; then
    echo "ok: $crate with the dependency feature depends on lindera-dependency"
  else
    echo "error: $crate with the dependency feature does not depend on lindera-dependency" >&2
    failed=1
  fi
done

exit "$failed"

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
# Usage:
#   scripts/check_dependency_crate.sh
#
# Builds the crate for wasm32-unknown-unknown and reads
# `cargo tree --locked -p lindera-dependency -e normal`, from the workspace
# root. Prints one line per check, and fails if the build fails or if the
# crate depends on another crate of the workspace than lindera-crf.

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

exit "$failed"

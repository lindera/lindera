#!/usr/bin/env bash
#
# Installs a lindera source gem with `gem install`, as a user would, and runs
# smoke_test.rb against it from outside any checkout.
#
# Usage:
#   install_and_test.sh [--setup] <path/to/lindera-X.Y.Z.gem>
#
#   --setup  First install what a bare Debian-based `ruby` image lacks: libclang
#            (apt-get, as root) and a Rust toolchain (rustup).
#
# Environment:
#   LINDERA_FEATURES    Cargo features for the extension build. Defaults to
#                       embed-ipadic; set it empty for the default build.
#   LINDERA_DICTIONARY  Dictionary for smoke_test.rb to load (a path or an
#                       embedded:// URI). Defaults to embedded://ipadic.
#   LINDERA_SOURCE_DIR  A Lindera checkout to compile the lindera crates from
#                       instead of crates.io, for testing unreleased changes.
#                       The [patch.crates-io] entry this needs goes into a
#                       temporary CARGO_HOME, removed on exit, so the caller's
#                       Cargo configuration is never modified (crates are
#                       downloaded afresh into it). Leave it unset to install
#                       exactly as users do.
#
# Examples:
#   make test-lindera-ruby-gem
#   docker run --rm -v "$PWD:/src:ro" ruby:3.3 \
#     bash /src/lindera-ruby/test/gem/install_and_test.sh --setup /src/lindera-ruby/pkg/lindera-X.Y.Z.gem

set -euo pipefail

setup=false
if [[ "${1:-}" == "--setup" ]]; then
  setup=true
  shift
fi
if [[ $# -ne 1 ]]; then
  echo "Usage: $(basename "$0") [--setup] <gem-file>" >&2
  exit 1
fi

gem_file="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "$setup" == true ]]; then
  # rb-sys generates its Ruby bindings with bindgen, which loads libclang.
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends libclang-dev >/dev/null
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --quiet --profile minimal --default-toolchain stable
  # shellcheck source=/dev/null
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi

if [[ -n "${LINDERA_SOURCE_DIR:-}" ]]; then
  source_dir="$(cd "$LINDERA_SOURCE_DIR" && pwd)"
  # Cargo reads [patch] only from a manifest or a config file, never from the
  # environment, and rb_sys cannot pass `--config` to cargo. Appending to the
  # caller's $CARGO_HOME/config.toml would patch every other project using it
  # and break it on a second run (duplicate table), so use a throwaway
  # CARGO_HOME instead. The rustup proxies still find the toolchain through
  # RUSTUP_HOME, which is left as is.
  CARGO_HOME="$(mktemp -d)"
  export CARGO_HOME
  trap 'rm -rf "$CARGO_HOME"' EXIT
  cat >"$CARGO_HOME/config.toml" <<TOML
[patch.crates-io]
lindera = { path = "$source_dir/lindera" }
lindera-binding-core = { path = "$source_dir/lindera-binding-core" }
TOML
  echo "Compiling the lindera crates from $source_dir instead of crates.io (CARGO_HOME=$CARGO_HOME)"
fi

export LINDERA_FEATURES="${LINDERA_FEATURES-embed-ipadic}"
cargo --version
gem install --no-document "$gem_file"

# Run from / so that nothing but the installed gem can satisfy `require`.
cd /
ruby "$script_dir/smoke_test.rb" "${LINDERA_DICTIONARY:-embedded://ipadic}"

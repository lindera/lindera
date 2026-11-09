#!/usr/bin/env bash
# Writes, for each dictionary named in DICTIONARIES, whether this job built it
# or reused it from the cache: built when its build_inputs.json under
# LINDERA_BUILD_DICTIONARY_CACHE_DIR is newer than the marker that the restore
# left, reused otherwise. A restored file keeps the time it had when it was
# saved, and a build writes build_inputs.json last (#1337).
#
# Usage: report.sh <marker file>
# Environment: DICTIONARIES (names separated by spaces),
# LINDERA_BUILD_DICTIONARY_CACHE_DIR.
set -u
marker="$1"
dir="${LINDERA_BUILD_DICTIONARY_CACHE_DIR:?LINDERA_BUILD_DICTIONARY_CACHE_DIR is not set}"
for name in $DICTIONARIES; do
  inputs=$(find "$dir" -path "*/lindera-$name/build_inputs.json" -print -quit 2>/dev/null)
  if [ -z "$inputs" ]; then
    echo "lindera-$name: not in the cache directory"
  elif [ "$inputs" -nt "$marker" ]; then
    echo "lindera-$name: built in this job"
  else
    echo "lindera-$name: reused from the cache"
  fi
done

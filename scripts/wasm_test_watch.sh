#!/usr/bin/env bash
#
# Runs the Node.js tests of lindera-wasm, and tells when the process that
# runs them does not exit after its tests have passed (#1486).
#
# `wasm-pack test --node` gives the tests to `wasm-bindgen-test-runner`,
# which runs them in Node.js and asks it to exit. On the runners of GitHub
# that process has stayed after the line of the result, until the timeout
# of the job. This script runs the tests, waits for that line, and gives
# the process a time to exit after it. When the time is over, it writes
# what the process is doing (its threads, and their stacks when gdb is
# there) and kills it.
#
# Usage:
#   scripts/wasm_test_watch.sh [--runs <n>] [--grace <seconds>] [--limit <seconds>]
#                              [--command <command>]
#     --runs    How many times the tests are run (default 1).
#     --grace   The seconds the process has to exit in after the line of the
#               result (default 120).
#     --limit   The seconds a run may take in all (default 900).
#     --command The command of a run, in place of `wasm-pack test --node`
#               with the features: for trying this script on a process
#               that stays.
#
# Run from the root of the repository. The features are WASM_FEATURES of the
# Makefile. Prints one line per run, EXITED, HUNG_AFTER_RESULT,
# HUNG_BEFORE_RESULT or FAILED, and a line of counts at the end. The exit
# code is 0 when every run exited, 2 when one hung, and 1 when one failed.

set -u

runs=1
grace=120
limit=900
command=""
while [ $# -gt 0 ]; do
  case "$1" in
  --runs)
    runs=$2
    shift 2
    ;;
  --grace)
    grace=$2
    shift 2
    ;;
  --limit)
    limit=$2
    shift 2
    ;;
  --command)
    command=$2
    shift 2
    ;;
  *)
    echo "error: unknown argument $1" >&2
    exit 1
    ;;
  esac
done

features=$(sed -n 's/^WASM_FEATURES = //p' Makefile)
if [ -z "$features" ]; then
  echo "error: WASM_FEATURES is not in the Makefile; run from the root of the repository" >&2
  exit 1
fi
if [ -z "$command" ]; then
  command="cd lindera-wasm && exec wasm-pack test --node --features='$features'"
fi
logs=$(mktemp -d)
echo "node $(node --version), $(wasm-pack --version), $(nproc) processors, features $features, NODE_ARGS '${NODE_ARGS:-}'"

# Writes what the processes of a run are doing: every process of the group,
# with its threads and their stacks.
describe() {
  local group=$1
  echo "--- the processes of the run"
  ps -o pid,ppid,pgid,nlwp,stat,etime,args -g "$group" | cut -c1-200
  # Every process of the group: Node.js names its main thread `MainThread`,
  # and that is what the name of its process reads as.
  local pid
  for pid in $(pgrep -g "$group"); do
    echo "--- the threads of $pid ($(tr '\0' ' ' < "/proc/$pid/cmdline" 2> /dev/null | cut -c1-80))"
    ps -L -o tid,stat,pcpu,comm -p "$pid"
    if ! command -v gdb > /dev/null; then
      echo "gdb is not installed: no stacks"
      continue
    fi
    echo "--- the stacks of $pid"
    # A process may trace its descendants only, where the kernel says so:
    # the runners of GitHub let sudo through without a password.
    local gdb=(gdb)
    if sudo -n true 2> /dev/null; then gdb=(sudo -n gdb); fi
    "${gdb[@]}" -p "$pid" -batch -ex 'set pagination off' -ex 'thread apply all bt 25' 2>&1 |
      grep -v '^\[New LWP\|^warning: \|^\[Thread debugging\|^Using host' | cut -c1-240
  done
}

exited=0
after=0
before=0
failed=0
for run in $(seq "$runs"); do
  log=$logs/run$run.log
  start=$SECONDS
  # A group of its own, so that the run can be described and killed whole.
  setsid bash -c "$command" > "$log" 2>&1 &
  child=$!
  result_at=""
  state=""
  while kill -0 "$child" 2> /dev/null; do
    sleep 1
    now=$((SECONDS - start))
    # `wasm-pack test --node` writes one line of result, when the tests in
    # Node.js are over.
    if [ -z "$result_at" ] && grep -q '^test result: ok\.' "$log"; then
      result_at=$now
    fi
    if [ -n "$result_at" ] && [ $((now - result_at)) -ge "$grace" ]; then
      state=HUNG_AFTER_RESULT
    elif [ "$now" -ge "$limit" ]; then
      state=HUNG_BEFORE_RESULT
    fi
    if [ -n "$state" ]; then
      echo "run $run: $state after $now s (the result was written at ${result_at:-no time} s)"
      describe "$child"
      kill -KILL -- "-$child" 2> /dev/null
      break
    fi
  done
  wait "$child" 2> /dev/null
  code=$?
  took=$((SECONDS - start))
  case "$state" in
  HUNG_AFTER_RESULT) after=$((after + 1)) ;;
  HUNG_BEFORE_RESULT)
    before=$((before + 1))
    tail -20 "$log"
    ;;
  *)
    if [ "$code" = 0 ]; then
      exited=$((exited + 1))
      echo "run $run: EXITED in $took s"
    else
      failed=$((failed + 1))
      echo "run $run: FAILED with $code in $took s"
      tail -30 "$log"
    fi
    ;;
  esac
done
echo "runs $runs: exited $exited, hung after the result $after, hung before the result $before, failed $failed"
rm -rf "$logs"
if [ "$failed" -gt 0 ]; then exit 1; fi
if [ $((after + before)) -gt 0 ]; then exit 2; fi
exit 0

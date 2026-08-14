#!/usr/bin/env bash
# Interleaved A/B runner for the per-binding bench scripts.
#
# Usage:
#   ab_interleaved.sh <label_a> <cmd_a> <label_b> <cmd_b> [rounds]
#
# Each command must print TSV lines "<workload>\t<value>" (the bench
# scripts in this directory do). The runner:
#   1. runs both commands with --verify and requires byte-identical output
#      per workload (correctness gate),
#   2. runs `rounds` rounds (default 8), alternating A->B and swapping the
#      order after the first half so slow drift cancels,
#   3. prints the per-round table and the min-of-round-minima per side,
#      with the relative difference of B vs A per workload.
#
# Run a null pair first (same command on both sides) and require the
# difference to be within ~2-3% before trusting a real A/B session.
# See README.md in this directory for the full protocol.

set -euo pipefail

if [ "$#" -lt 4 ]; then
    echo "usage: $0 <label_a> <cmd_a> <label_b> <cmd_b> [rounds]" >&2
    exit 2
fi

LABEL_A=$1
CMD_A=$2
LABEL_B=$3
CMD_B=$4
ROUNDS=${5:-8}

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

echo "== correctness gate (--verify) =="
bash -c "$CMD_A --verify" >"$workdir/verify_a.tsv"
bash -c "$CMD_B --verify" >"$workdir/verify_b.tsv"
if ! diff -u "$workdir/verify_a.tsv" "$workdir/verify_b.tsv" >"$workdir/verify.diff"; then
    # A missing workload on one side (e.g. baseline without the new API) is
    # tolerated per workload; any differing shared line is fatal.
    while IFS=$'\t' read -r workload _; do
        a_line=$(grep "^${workload}	" "$workdir/verify_a.tsv" || true)
        b_line=$(grep "^${workload}	" "$workdir/verify_b.tsv" || true)
        if [ -n "$a_line" ] && [ -n "$b_line" ] && [ "$a_line" != "$b_line" ]; then
            echo "FATAL: output mismatch for workload '$workload'" >&2
            diff -u <(echo "$a_line") <(echo "$b_line") >&2 || true
            exit 1
        fi
    done <"$workdir/verify_a.tsv"
    echo "note: workload sets differ between sides (tolerated); shared workloads match"
else
    echo "outputs identical"
fi

run_side() {
    # $1 = label, $2 = command, $3 = round index
    bash -c "$2" | while IFS=$'\t' read -r workload value; do
        printf '%s\t%s\t%s\t%s\n' "$3" "$1" "$workload" "$value"
    done >>"$workdir/results.tsv"
}

echo "== interleaved rounds: $ROUNDS (order swaps after round $((ROUNDS / 2))) =="
for round in $(seq 1 "$ROUNDS"); do
    if [ "$round" -le $((ROUNDS / 2)) ]; then
        run_side "$LABEL_A" "$CMD_A" "$round"
        run_side "$LABEL_B" "$CMD_B" "$round"
    else
        run_side "$LABEL_B" "$CMD_B" "$round"
        run_side "$LABEL_A" "$CMD_A" "$round"
    fi
    echo "round $round done"
done

echo
echo "== per-round results (round / side / workload / us_per_call) =="
column -t "$workdir/results.tsv"

echo
echo "== min of round minima =="
awk -F'\t' -v la="$LABEL_A" -v lb="$LABEL_B" '
    {
        key = $2 SUBSEP $3
        if (!(key in best) || $4 + 0 < best[key]) best[key] = $4 + 0
        workloads[$3] = 1
    }
    END {
        printf "%-12s %-12s %-12s %s\n", "workload", la, lb, "B vs A"
        for (w in workloads) {
            a = best[la SUBSEP w]
            b = best[lb SUBSEP w]
            if (a == "" || b == "") {
                printf "%-12s %-12s %-12s %s\n", w, (a == "" ? "-" : a), (b == "" ? "-" : b), "n/a"
            } else {
                printf "%-12s %-12.3f %-12.3f %+.1f%%\n", w, a, b, (b - a) / a * 100
            }
        }
    }
' "$workdir/results.tsv"

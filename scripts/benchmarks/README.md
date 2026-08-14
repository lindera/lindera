# Interleaved A/B benchmark harness

Reproducible harness for the interleaved min-of-N protocol used by the
performance PR series (#883-#888, #913-#915 / #881). Earlier campaigns kept
this protocol only in commit messages; committing it here makes the numbers
reproducible from a clean checkout.

## Protocol

1. **Two sides, two checkouts.** Build side A (baseline) and side B
   (candidate) from separate `git worktree` checkouts so neither build
   disturbs the other. Bindings are built per side (e.g. two Python venvs
   with `maturin develop --release`, two `napi build --release` output
   directories).
2. **Correctness gate.** Before timing, run every bench script with
   `--verify` on both sides and require byte-identical output. If the
   outputs differ, do not measure.
3. **Null-pair test.** Run the interleaved procedure with A on both sides
   first. If the reported difference exceeds ±2-3%, the session is too
   noisy (background load, thermal state) — fix the environment before
   measuring, or discard the session.
4. **Interleaved rounds.** `ab_interleaved.sh` runs N rounds (default 8),
   alternating A→B and swapping the order after the first half so slow
   drift (thermal, cache) cancels. Each script invocation reports the
   minimum µs/call over `BENCH_INNER` repetitions (min-of-10 by default).
5. **Report.** The reported statistic is the **min of round minima** per
   side; the per-round table is printed so the spread is visible. Record
   both commit hashes, the CPU model, and the protocol parameters next to
   any published number.

## Usage

```sh
# 1. Prepare the baseline worktree (example: pre-series baseline):
git worktree add /tmp/lindera-baseline <baseline-commit>

# 2. Build both sides for the binding you measure (see per-language notes).

# 3. Correctness gate + null pair + interleaved run:
scripts/benchmarks/ab_interleaved.sh \
  "A" "python3 scripts/benchmarks/bench_python.py" \
  "B" "python3 scripts/benchmarks/bench_python.py" \
  8
```

The two command strings usually differ in which environment they activate
(e.g. `VIRTUAL_ENV=... .../python bench_python.py` for two venvs, or `node`
against two build output directories via `BENCH_LINDERA_DIR`).

## Per-language notes

| Language | Build per side | Bench script |
| --- | --- | --- |
| Python | `maturin develop --release --features embed-ipadic` into a per-side venv | `bench_python.py` |
| Node.js | `npx napi build --platform --release --features embed-ipadic` per side; point `BENCH_LINDERA_DIR` at the checkout | `bench_nodejs.mjs` |
| Ruby | `bundle exec rake compile` per side (`LINDERA_FEATURES=embed-ipadic`) | `bench_ruby.rb` |
| PHP | `cargo build -p lindera-php --release --features embed-ipadic`; pass the extension path via `BENCH_PHP_EXT` | `bench_php.php` |
| WASM | not measured in a browser (noise-dominated); covered by the Rust-level `lindera-binding-core` criterion bench instead | — |

## Environment variables

| Variable | Default | Meaning |
| --- | --- | --- |
| `BENCH_TEXT` | `すもももももももものうち` | Input text (short, per-call workload) |
| `BENCH_CALLS` | `2000` | Calls per timing sample |
| `BENCH_INNER` | `10` | Samples per invocation; the minimum is reported |
| `BENCH_LINDERA_DIR` | — | (Node.js) checkout directory whose build artifacts to load |
| `BENCH_PHP_EXT` | — | (PHP) path to `liblindera_php.so` |

Each bench script prints one TSV line per workload: `<workload>\t<min µs/call>`.
With `--verify` it instead prints the token surfaces for the input, one
workload per line, for the byte-identical gate.

These scripts are development tools; they are not part of any published
package (Python sdist, gem, npm, composer archives all package only their
own binding directories).

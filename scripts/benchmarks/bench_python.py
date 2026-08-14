#!/usr/bin/env python3
"""Per-call benchmark for lindera-python (interleaved A/B harness member).

Prints one TSV line per workload: "<workload>\t<min microseconds/call>".
With --verify, prints the token surfaces instead (for the byte-identical
correctness gate). See scripts/benchmarks/README.md for the protocol.
"""

import os
import sys
import time

from lindera import Tokenizer, load_dictionary

TEXT = os.environ.get("BENCH_TEXT", "すもももももももものうち")
CALLS = int(os.environ.get("BENCH_CALLS", "2000"))
INNER = int(os.environ.get("BENCH_INNER", "10"))


def us_per_call(fn):
    start = time.perf_counter()
    for _ in range(CALLS):
        fn(TEXT)
    return (time.perf_counter() - start) / CALLS * 1e6


def min_us(fn):
    return min(us_per_call(fn) for _ in range(INNER))


def main():
    dictionary = load_dictionary("embedded://ipadic")
    tokenizer = Tokenizer(dictionary, mode="normal")

    has_surfaces = hasattr(tokenizer, "tokenize_surfaces")

    if "--verify" in sys.argv:
        surfaces = [t.surface for t in tokenizer.tokenize(TEXT)]
        print("tokenize\t" + "\t".join(surfaces))
        if has_surfaces:
            print("surfaces\t" + "\t".join(tokenizer.tokenize_surfaces(TEXT)))
        return

    print(f"tokenize\t{min_us(tokenizer.tokenize):.3f}")
    if has_surfaces:
        print(f"surfaces\t{min_us(tokenizer.tokenize_surfaces):.3f}")


if __name__ == "__main__":
    main()

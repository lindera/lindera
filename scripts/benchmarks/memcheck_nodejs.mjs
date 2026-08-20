#!/usr/bin/env node
// Memory-regression check for lindera-nodejs (see #930).
//
// Tokenizes a large text in a tight synchronous loop with no event-loop
// yields — the workload from #922 where class-instance returns accumulated
// native memory — and asserts that RSS growth flattens out instead of
// climbing linearly. Exits non-zero on regression.
//
// Must be run with --expose-gc:
//   node --expose-gc scripts/benchmarks/memcheck_nodejs.mjs
//
// Environment:
//   BENCH_LINDERA_DIR  checkout whose build artifacts to load (default: this repo)
//   MEMCHECK_ITER      measured iterations (default 24)
//   MEMCHECK_WARMUP    warmup iterations (default 4)
//   MEMCHECK_CHARS     size of the generated input text (default 100000)

import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { measureRssGrowth, reportVerdict } from "./memcheck_common.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const linderaDir = process.env.BENCH_LINDERA_DIR ?? path.join(here, "..", "..");
const require = createRequire(import.meta.url);
const lindera = require(path.join(linderaDir, "lindera-nodejs", "index.js"));

const ITERATIONS = Number(process.env.MEMCHECK_ITER ?? "24");
const WARMUP = Number(process.env.MEMCHECK_WARMUP ?? "4");
const CHARS = Number(process.env.MEMCHECK_CHARS ?? "100000");

// Repeat a sentence rather than one long run of the same character, so the
// lattice does real work per call and the token count scales with the input.
const SENTENCE = "関西国際空港限定トートバッグを買った。";
const TEXT = SENTENCE.repeat(Math.ceil(CHARS / SENTENCE.length)).slice(
  0,
  CHARS,
);

const dictionary = lindera.loadDictionary("embedded://ipadic");
const tokenizer = new lindera.Tokenizer(dictionary, "normal");

// Sanity check: the loop below must actually produce tokens, otherwise the
// measurement would pass trivially.
const probe = tokenizer.tokenize(TEXT);
if (!Array.isArray(probe) || probe.length === 0) {
  console.error("FAIL: tokenize returned no tokens; nothing was measured");
  process.exit(1);
}
console.log(`nodejs\ttokens-per-call\t${probe.length}`);

const growth = measureRssGrowth({
  tokenizeOnce: () => {
    tokenizer.tokenize(TEXT);
  },
  iterations: ITERATIONS,
  warmup: WARMUP,
});

const passed = reportVerdict({
  label: "nodejs",
  growth,
  // Accumulation keeps the ratio near or above 1.0; a fixed binding lands
  // near 0. Half leaves generous headroom for allocator noise.
  tolerance: 0.5,
  ceilingBytes: 512 * 1024 * 1024,
});

process.exit(passed ? 0 : 1);

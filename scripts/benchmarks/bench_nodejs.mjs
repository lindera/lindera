#!/usr/bin/env node
// Per-call benchmark for lindera-nodejs (interleaved A/B harness member).
//
// Prints one TSV line per workload: "<workload>\t<min microseconds/call>".
// With --verify, prints the token surfaces instead (for the byte-identical
// correctness gate). BENCH_LINDERA_DIR selects which checkout's build
// artifacts to load (defaults to the repo this script lives in).
// See scripts/benchmarks/README.md for the protocol.

import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const linderaDir =
  process.env.BENCH_LINDERA_DIR ?? path.join(here, "..", "..");
const require = createRequire(import.meta.url);
const lindera = require(path.join(linderaDir, "lindera-nodejs", "index.js"));

const TEXT = process.env.BENCH_TEXT ?? "すもももももももものうち";
const CALLS = Number(process.env.BENCH_CALLS ?? "2000");
const INNER = Number(process.env.BENCH_INNER ?? "10");

function usPerCall(fn) {
  const start = process.hrtime.bigint();
  for (let i = 0; i < CALLS; i++) {
    fn(TEXT);
  }
  const elapsedNs = Number(process.hrtime.bigint() - start);
  return elapsedNs / CALLS / 1000;
}

function minUs(fn) {
  let best = Infinity;
  for (let i = 0; i < INNER; i++) {
    best = Math.min(best, usPerCall(fn));
  }
  return best;
}

const dictionary = lindera.loadDictionary("embedded://ipadic");
const tokenizer = new lindera.Tokenizer(dictionary, "normal");
const hasSurfaces = typeof tokenizer.tokenizeSurfaces === "function";

if (process.argv.includes("--verify")) {
  const surfaces = tokenizer.tokenize(TEXT).map((t) => t.surface);
  console.log("tokenize\t" + surfaces.join("\t"));
  if (hasSurfaces) {
    console.log("surfaces\t" + tokenizer.tokenizeSurfaces(TEXT).join("\t"));
  }
} else {
  console.log(`tokenize\t${minUs((t) => tokenizer.tokenize(t)).toFixed(3)}`);
  if (hasSurfaces) {
    console.log(
      `surfaces\t${minUs((t) => tokenizer.tokenizeSurfaces(t)).toFixed(3)}`,
    );
  }
}

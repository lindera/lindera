#!/usr/bin/env node
// Memory-regression check for lindera-wasm (see #930).
//
// Same workload and assertion as memcheck_nodejs.mjs, against a
// `wasm-pack build --target nodejs` artifact so the check runs in a plain
// Node process rather than a browser. Exits non-zero on regression.
//
// Must be run with --expose-gc, and needs a nodejs-target build first:
//   cd lindera-wasm && wasm-pack build --target nodejs --features embed-ipadic --out-dir pkg-node
//   node --expose-gc scripts/benchmarks/memcheck_wasm.mjs
//
// Environment:
//   MEMCHECK_WASM_PKG  package directory to load (default: lindera-wasm/pkg-node)
//   MEMCHECK_ITER      measured iterations (default 24)
//   MEMCHECK_WARMUP    warmup iterations (default 4)
//   MEMCHECK_CHARS     size of the generated input text (default 100000)

import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { measureRssGrowth, reportVerdict } from "./memcheck_common.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.join(here, "..", "..");
const pkgDir =
  process.env.MEMCHECK_WASM_PKG ??
  path.join(repoRoot, "lindera-wasm", "pkg-node");

const require = createRequire(import.meta.url);
const lindera = require(pkgDir);

const ITERATIONS = Number(process.env.MEMCHECK_ITER ?? "24");
const WARMUP = Number(process.env.MEMCHECK_WARMUP ?? "4");
const CHARS = Number(process.env.MEMCHECK_CHARS ?? "100000");

const SENTENCE = "関西国際空港限定トートバッグを買った。";
const TEXT = SENTENCE.repeat(Math.ceil(CHARS / SENTENCE.length)).slice(
  0,
  CHARS,
);

const builder = new lindera.TokenizerBuilder();
builder.setMode("normal");
builder.setDictionary("embedded://ipadic");
const tokenizer = builder.build();

// Sanity check: the loop below must actually produce tokens, otherwise the
// measurement would pass trivially.
const probe = tokenizer.tokenize(TEXT);
if (!Array.isArray(probe) || probe.length === 0) {
  console.error("FAIL: tokenize returned no tokens; nothing was measured");
  process.exit(1);
}
console.log(`wasm\ttokens-per-call\t${probe.length}`);

const growth = measureRssGrowth({
  tokenizeOnce: () => {
    tokenizer.tokenize(TEXT);
  },
  iterations: ITERATIONS,
  warmup: WARMUP,
});

const passed = reportVerdict({
  label: "wasm",
  growth,
  tolerance: 0.5,
  ceilingBytes: 512 * 1024 * 1024,
});

process.exit(passed ? 0 : 1);

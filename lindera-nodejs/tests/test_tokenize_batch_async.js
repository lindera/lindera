// The batch methods of Tokenizer that return a Promise (lindera/lindera#1406):
// the texts are tokenized on a thread of libuv's pool while the event loop
// runs, and the results are those of the synchronous batch methods.
const assert = require("node:assert");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { describe, it } = require("node:test");
const util = require("node:util");

const lindera = require("../index.js");

// Matches the guard in test_tokenize_ipadic.js.
const hasEmbeddedIpadic = (() => {
  try {
    lindera.loadDictionary("embedded://ipadic");
    return true;
  } catch {
    return false;
  }
})();

// `__panicInTaskForTest` exists only in a build with the `test-hooks`
// feature, which `make test-lindera-nodejs` enables and a release build never
// does.
const hasTestHooks = typeof lindera.__panicInTaskForTest === "function";

// The lines of resources/bocchan.txt and an empty text: enough input for a
// batch on several threads, and for one that takes a while on one.
function texts() {
  const file = path.join(__dirname, "..", "..", "resources", "bocchan.txt");
  return fs.readFileSync(file, "utf8").split("\n").concat([""]);
}

function tokenizer() {
  return new lindera.Tokenizer(
    lindera.loadDictionary("embedded://ipadic"),
    lindera.Mode.normal(),
  );
}

// The error that `call` throws.
function thrown(call) {
  try {
    call();
  } catch (error) {
    return error;
  }
  return assert.fail("the call did not throw");
}

describe(
  "Tokenizer batch methods that return a Promise",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    for (const maxThreads of [undefined, null, 1, 2, 8]) {
      it(`tokenizeBatchAsync resolves to tokenizeBatch's tokens (maxThreads ${maxThreads})`, async () => {
        const t = tokenizer();
        const all = texts();
        const pending = t.tokenizeBatchAsync(all, maxThreads);
        assert.ok(pending instanceof Promise);
        assert.deepStrictEqual(await pending, t.tokenizeBatch(all, maxThreads));
      });
    }

    it("tokenizeSurfacesBatchAsync resolves to tokenizeSurfacesBatch's surfaces", async () => {
      const t = tokenizer();
      const all = texts();
      const expected = t.tokenizeSurfacesBatch(all);
      assert.deepStrictEqual(
        await t.tokenizeSurfacesBatchAsync(all, 4),
        expected,
      );
      assert.deepStrictEqual(await t.tokenizeSurfacesBatchAsync(all), expected);
    });

    it("tokenizeNbestBatchAsync resolves to tokenizeNbestBatch's results", async () => {
      const t = tokenizer();
      const some = texts().slice(0, 60);
      const expected = t.tokenizeNbestBatch(some, 3, true, 5000);
      assert.deepStrictEqual(
        await t.tokenizeNbestBatchAsync(some, 3, true, 5000, 4),
        expected,
      );
      assert.deepStrictEqual(
        await t.tokenizeNbestBatchAsync(some, 3),
        t.tokenizeNbestBatch(some, 3),
      );
    });

    it("tokenizeBatchAsync resolves an empty array to an empty array", async () => {
      assert.deepStrictEqual(await tokenizer().tokenizeBatchAsync([]), []);
    });

    // The batch runs on one thread so that the tokenization takes most of
    // the time from the call to the settlement. The event loop runs during
    // it: the last setImmediate callback before the settlement comes in the
    // second half of that time. Were the texts tokenized on the JavaScript
    // thread, the callbacks would run only before it, at the start. The
    // surfaces take little time to become strings, which keeps the share of
    // the JavaScript thread's own part small.
    it("lets the event loop run while the texts are tokenized", async () => {
      const t = tokenizer();
      const all = texts();
      let ticks = 0;
      let lastTick = 0;
      let settled = false;
      const tick = () => {
        ticks += 1;
        lastTick = performance.now();
        if (!settled) {
          setImmediate(tick);
        }
      };
      const start = performance.now();
      const pending = t.tokenizeSurfacesBatchAsync(all, 1);
      setImmediate(tick);
      const batch = await pending;
      const end = performance.now();
      settled = true;
      const share = (lastTick - start) / (end - start);
      assert.ok(
        share >= 0.5,
        `the last of ${ticks} event loop ticks came at ${(lastTick - start).toFixed(1)} ms ` +
          `of ${(end - start).toFixed(1)} ms`,
      );
      assert.strictEqual(batch.length, all.length);
    });

    // An argument out of its range rejects the Promise with the error the
    // synchronous method throws, instead of throwing.
    const outOfRange = [
      [
        "tokenizeBatchAsync, maxThreads 0",
        (t) => t.tokenizeBatchAsync(["東京"], 0),
        (t) => t.tokenizeBatch(["東京"], 0),
      ],
      [
        "tokenizeBatchAsync, maxThreads -1",
        (t) => t.tokenizeBatchAsync(["東京"], -1),
        (t) => t.tokenizeBatch(["東京"], -1),
      ],
      [
        "tokenizeBatchAsync, maxThreads 1.5",
        (t) => t.tokenizeBatchAsync(["東京"], 1.5),
        (t) => t.tokenizeBatch(["東京"], 1.5),
      ],
      [
        "tokenizeSurfacesBatchAsync, maxThreads 0",
        (t) => t.tokenizeSurfacesBatchAsync(["東京"], 0),
        (t) => t.tokenizeSurfacesBatch(["東京"], 0),
      ],
      [
        "tokenizeNbestBatchAsync, n -1",
        (t) => t.tokenizeNbestBatchAsync(["東京"], -1),
        (t) => t.tokenizeNbestBatch(["東京"], -1),
      ],
      [
        "tokenizeNbestBatchAsync, costThreshold 1e300",
        (t) => t.tokenizeNbestBatchAsync(["東京"], 2, false, 1e300),
        (t) => t.tokenizeNbestBatch(["東京"], 2, false, 1e300),
      ],
      [
        "tokenizeNbestBatchAsync, maxThreads 0",
        (t) => t.tokenizeNbestBatchAsync(["東京"], 2, false, null, 0),
        (t) => t.tokenizeNbestBatch(["東京"], 2, false, null, 0),
      ],
    ];
    for (const [label, call, sync] of outOfRange) {
      it(`${label}: rejects as the synchronous method throws`, async () => {
        const t = tokenizer();
        const expected = thrown(() => sync(t));
        assert.strictEqual(expected.code, "InvalidArg");
        let pending;
        assert.doesNotThrow(() => {
          pending = call(t);
        }, `${label} threw instead of rejecting`);
        await assert.rejects(pending, (error) => {
          assert.ok(error instanceof Error);
          assert.strictEqual(error.code, expected.code);
          assert.strictEqual(error.message, expected.message);
          return true;
        });
      });
    }

    // napi converts the arguments before the method runs, so an argument of
    // the wrong type throws, as it does for every napi method.
    it("tokenizeBatchAsync throws for a string instead of an array", () => {
      assert.throws(() => tokenizer().tokenizeBatchAsync("東京"));
    });

    // A batch runs on the tokenizer's batch workers, behind a lock of their
    // own: a single-text call on the same tokenizer still gives its result,
    // and two batches at once wait for each other and give theirs.
    it("gives every result when a single-text call and two batches share a tokenizer", async () => {
      const t = tokenizer();
      const all = texts();
      const expected = t.tokenizeBatch(all, 2);
      const single = tokenizer().tokenize("東京スカイツリー");
      const first = t.tokenizeBatchAsync(all, 2);
      const second = t.tokenizeBatchAsync(all, 2);
      assert.deepStrictEqual(t.tokenize("東京スカイツリー"), single);
      const [a, b] = await Promise.all([first, second]);
      assert.deepStrictEqual(a, expected);
      assert.deepStrictEqual(b, expected);
    });
  },
);

// napi calls a task's `compute` without catching a panic; every batch task's
// `compute` goes through `compute_batch`, which makes a panic the error that
// `#[napi(catch_unwind)]` makes. Without it, Node.js would abort.
describe(
  "A panic in a task",
  { skip: !hasTestHooks && "built without the test-hooks feature" },
  () => {
    const index = path.join(__dirname, "..", "index.js");
    const message = "lindera-nodejs test panic in a task";

    // The panic happens in a child, so that a panic that aborts kills the
    // child alone, and the assertions below say how it ended.
    const childScript = `
      const lindera = require(${JSON.stringify(index)});
      lindera.__panicInTaskForTest(${JSON.stringify(message)}).then(
        () => console.log(JSON.stringify({ resolved: true })),
        (error) => {
          // The binding goes on working after the panic.
          const version = lindera.version();
          console.log(JSON.stringify({
            rejected: true,
            isError: error instanceof Error,
            code: error && error.code,
            message: error && error.message,
            version,
          }));
        },
      );
    `;

    it("rejects the Promise instead of aborting Node.js", () => {
      const child = spawnSync(process.execPath, ["-e", childScript], {
        encoding: "utf8",
        timeout: 60_000,
      });
      const ended =
        `exit code ${child.status}, signal ${child.signal}, ` +
        `standard error:\n${(child.stderr || "").slice(-2000)}`;
      assert.strictEqual(child.error, undefined, ended);
      assert.strictEqual(child.signal, null, ended);
      assert.strictEqual(child.status, 0, ended);
      assert.deepStrictEqual(
        JSON.parse(child.stdout),
        {
          rejected: true,
          isError: true,
          code: "GenericFailure",
          message,
          version: lindera.version(),
        },
        ended,
      );
    });
  },
);

// A batch whose results do not fit in one part is made in parts, with a turn
// of the event loop between two parts (lindera/lindera#1408). The test hooks
// set the budget of a part, count the parts, and make a part fail; they exist
// only in a build with the `test-hooks` feature.
describe(
  "Making the results of a batch in parts",
  {
    skip:
      (!hasEmbeddedIpadic && "embedded://ipadic not available") ||
      (!hasTestHooks && "built without the test-hooks feature"),
  },
  () => {
    const index = path.join(__dirname, "..", "index.js");
    const bocchan = path.join(
      __dirname,
      "..",
      "..",
      "resources",
      "bocchan.txt",
    );

    // 40 lines, an empty text, and the 40 lines as one text: parts that end
    // inside a text, between texts, and after an empty one.
    function partTexts() {
      const lines = fs.readFileSync(bocchan, "utf8").split("\n").slice(0, 40);
      return lines.concat(["", lines.join("")]);
    }

    // Runs `call` with the budget of a part set to `units`, then puts the
    // default back.
    async function withBudget(units, call) {
      lindera.__setBatchPartBudgetForTest(units);
      try {
        return await call();
      } finally {
        lindera.__setBatchPartBudgetForTest(null);
      }
    }

    // Runs `script` in a child Node.js, so that a failure that ends the
    // process ends the child alone; returns what it printed as JSON.
    function runChild(script) {
      const child = spawnSync(process.execPath, ["-e", script], {
        encoding: "utf8",
        timeout: 120_000,
      });
      const ended =
        `exit code ${child.status}, signal ${child.signal}, ` +
        `standard output:\n${(child.stdout || "").slice(-2000)}\n` +
        `standard error:\n${(child.stderr || "").slice(-2000)}`;
      assert.strictEqual(child.error, undefined, ended);
      assert.strictEqual(child.signal, null, ended);
      assert.strictEqual(child.status, 0, ended);
      return { result: JSON.parse(child.stdout), ended };
    }

    // The start of every child script: the binding, a tokenizer, the lines.
    const childPrelude = `
      const fs = require("node:fs");
      const lindera = require(${JSON.stringify(index)});
      const t = new lindera.Tokenizer(
        lindera.loadDictionary("embedded://ipadic"),
        lindera.Mode.normal(),
      );
      const lines = fs.readFileSync(${JSON.stringify(bocchan)}, "utf8").split("\\n").slice(0, 40);
      const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
    `;

    // Checks that two batches are deeply and strictly equal, text by text,
    // and names the first text and element that differ: the diff that
    // `assert.deepStrictEqual` makes of two large batches can take longer
    // than the test may run.
    function assertSameBatch(actual, expected, label) {
      assert.strictEqual(actual.length, expected.length, `${label}: texts`);
      for (let i = 0; i < expected.length; i += 1) {
        if (util.isDeepStrictEqual(actual[i], expected[i])) {
          continue;
        }
        const got = Array.isArray(actual[i]) ? actual[i] : [];
        let j = 0;
        while (
          j < expected[i].length &&
          util.isDeepStrictEqual(got[j], expected[i][j])
        ) {
          j += 1;
        }
        assert.fail(
          `${label}: text ${i}, element ${j} of ${expected[i].length} ` +
            `(${got.length} made): ${util.inspect(got[j], { depth: 1 })} ` +
            `instead of ${util.inspect(expected[i][j], { depth: 1 })}`,
        );
      }
    }

    for (const budget of [1, 7, 50]) {
      it(`gives the synchronous results with parts of ${budget}`, async () => {
        const t = tokenizer();
        const all = partTexts();
        await withBudget(budget, async () => {
          assertSameBatch(
            await t.tokenizeBatchAsync(all),
            t.tokenizeBatch(all),
            "tokenizeBatchAsync",
          );
          assertSameBatch(
            await t.tokenizeSurfacesBatchAsync(all),
            t.tokenizeSurfacesBatch(all),
            "tokenizeSurfacesBatchAsync",
          );
          assertSameBatch(
            await t.tokenizeNbestBatchAsync(all, 3, true),
            t.tokenizeNbestBatch(all, 3, true),
            "tokenizeNbestBatchAsync",
          );
        });
      });
    }

    it("makes one token or one empty text per part with parts of 1", async () => {
      const t = tokenizer();
      const all = partTexts();
      const expected = t
        .tokenizeBatch(all)
        .reduce((sum, tokens) => sum + Math.max(1, tokens.length), 0);
      await withBudget(1, async () => {
        for (const call of [
          "tokenizeBatchAsync",
          "tokenizeSurfacesBatchAsync",
        ]) {
          const before = lindera.__batchPartsMadeForTest();
          await t[call](all);
          assert.strictEqual(
            lindera.__batchPartsMadeForTest() - before,
            expected,
            call,
          );
        }
      });
    });

    it("makes a batch that fits in one part at once", async () => {
      const t = tokenizer();
      const before = lindera.__batchPartsMadeForTest();
      assert.deepStrictEqual(
        await t.tokenizeBatchAsync(["東京"]),
        t.tokenizeBatch(["東京"]),
      );
      assert.strictEqual(lindera.__batchPartsMadeForTest(), before);
    });

    it("lets timers and immediates run between two parts", async () => {
      const t = tokenizer();
      const all = partTexts();
      await withBudget(1, async () => {
        const before = lindera.__batchPartsMadeForTest();
        let settled = false;
        let ticksInParts = 0;
        let timerSet = false;
        let timerFired = false;
        const tick = () => {
          const made = lindera.__batchPartsMadeForTest() - before;
          if (made > 0 && !settled) {
            ticksInParts += 1;
            if (!timerSet) {
              timerSet = true;
              setTimeout(() => {
                timerFired = !settled;
              }, 0);
            }
          }
          if (!settled) {
            setImmediate(tick);
          }
        };
        const pending = t.tokenizeBatchAsync(all);
        setImmediate(tick);
        await pending;
        settled = true;
        const parts = lindera.__batchPartsMadeForTest() - before;
        assert.ok(parts > 100, `${parts} parts`);
        assert.ok(
          ticksInParts >= parts - 1,
          `${ticksInParts} ticks between ${parts} parts`,
        );
        assert.ok(
          timerFired,
          "a timer set at the first part ran before the promise settled",
        );
      });
    });

    for (const kind of ["error", "panic"]) {
      it(`rejects the Promise on ${kind === "error" ? "an error" : "a panic"} in a later part`, () => {
        const { result, ended } = runChild(`${childPrelude}
          lindera.__setBatchPartBudgetForTest(1);
          lindera.__failBatchPartForTest(2, ${JSON.stringify(kind)});
          const before = lindera.__batchPartsMadeForTest();
          t.tokenizeBatchAsync(lines).then(
            () => console.log(JSON.stringify({ resolved: true })),
            (error) => {
              // No part after the one that failed, then the binding goes on
              // working.
              setTimeout(async () => {
                const parts = lindera.__batchPartsMadeForTest() - before;
                lindera.__setBatchPartBudgetForTest(7);
                const again = await t.tokenizeBatchAsync(lines);
                console.log(JSON.stringify({
                  rejected: true,
                  isError: error instanceof Error,
                  code: error && error.code,
                  message: error && error.message,
                  parts,
                  again: same(again, t.tokenizeBatch(lines)),
                }));
              }, 50);
            },
          );
        `);
        assert.deepStrictEqual(
          result,
          {
            rejected: true,
            isError: true,
            code: "GenericFailure",
            message: "lindera-nodejs test failure in part 2",
            parts: 2,
            again: true,
          },
          ended,
        );
      });
    }

    it("rejects the Promise with an exception thrown in a later part", () => {
      const { result, ended } = runChild(`${childPrelude}
        lindera.__setBatchPartBudgetForTest(1);
        lindera.__failBatchPartForTest(2, "throw");
        t.tokenizeBatchAsync(lines).then(
          () => console.log(JSON.stringify({ resolved: true })),
          // The exception thrown has no code, unlike the error of a part
          // that fails (GenericFailure).
          (error) => console.log(JSON.stringify({
            rejected: true,
            code: (error && error.code) ?? null,
            message: error && error.message,
          })),
        );
      `);
      assert.deepStrictEqual(
        result,
        {
          rejected: true,
          code: null,
          message: "lindera-nodejs test failure in part 2",
        },
        ended,
      );
    });

    it("rejects the Promise on a panic in resolve", () => {
      const { result, ended } = runChild(`${childPrelude}
        lindera.__failBatchPartForTest(0, "panic");
        t.tokenizeBatchAsync(["東京"]).then(
          () => console.log(JSON.stringify({ resolved: true })),
          (error) => console.log(JSON.stringify({
            rejected: true,
            code: error && error.code,
            message: error && error.message,
          })),
        );
      `);
      assert.deepStrictEqual(
        result,
        {
          rejected: true,
          code: "GenericFailure",
          message: "lindera-nodejs test failure in part 0",
        },
        ended,
      );
    });

    // A worker that starts a batch in parts of 1 and says so once a part
    // has been made; its parent then terminates it.
    const workerCode = `
      const { parentPort, workerData } = require("node:worker_threads");
      const lindera = require(workerData.index);
      const t = new lindera.Tokenizer(
        lindera.loadDictionary("embedded://ipadic"),
        lindera.Mode.normal(),
      );
      const lines = require("node:fs").readFileSync(workerData.bocchan, "utf8").split("\\n");
      lindera.__setBatchPartBudgetForTest(1);
      const before = lindera.__batchPartsMadeForTest();
      t.tokenizeBatchAsync(lines);
      const wait = () => {
        if (lindera.__batchPartsMadeForTest() > before) {
          parentPort.postMessage("started");
        } else {
          setImmediate(wait);
        }
      };
      setImmediate(wait);
    `;

    it("survives workers terminated with parts pending", () => {
      const { result, ended } = runChild(`${childPrelude}
        const { Worker } = require("node:worker_threads");
        const workerCode = ${JSON.stringify(workerCode)};
        const workerData = {
          index: ${JSON.stringify(index)},
          bocchan: ${JSON.stringify(bocchan)},
        };
        (async () => {
          for (let i = 0; i < 10; i += 1) {
            const worker = new Worker(workerCode, { eval: true, workerData });
            await new Promise((resolve, reject) => {
              worker.once("message", resolve);
              worker.once("error", reject);
            });
            await worker.terminate();
          }
          lindera.__setBatchPartBudgetForTest(7);
          const after = await t.tokenizeBatchAsync(lines);
          console.log(JSON.stringify({ same: same(after, t.tokenizeBatch(lines)) }));
        })().catch((error) => {
          console.log(JSON.stringify({ error: String(error) }));
        });
      `);
      assert.deepStrictEqual(result, { same: true }, ended);
    });

    it("gives the synchronous results to two batches in parts at once", async () => {
      const t = tokenizer();
      const all = partTexts();
      await withBudget(7, async () => {
        const [a, b] = await Promise.all([
          t.tokenizeBatchAsync(all),
          t.tokenizeSurfacesBatchAsync(all),
        ]);
        assertSameBatch(a, t.tokenizeBatch(all), "tokenizeBatchAsync");
        assertSameBatch(
          b,
          t.tokenizeSurfacesBatch(all),
          "tokenizeSurfacesBatchAsync",
        );
      });
    });

    it("settles without setImmediate, making the rest at once", () => {
      const { result, ended } = runChild(`${childPrelude}
        lindera.__setBatchPartBudgetForTest(1);
        const before = lindera.__batchPartsMadeForTest();
        const pending = t.tokenizeBatchAsync(lines);
        const saved = globalThis.setImmediate;
        globalThis.setImmediate = undefined;
        pending.then(
          (batch) => {
            globalThis.setImmediate = saved;
            console.log(JSON.stringify({
              same: same(batch, t.tokenizeBatch(lines)),
              parts: lindera.__batchPartsMadeForTest() - before,
            }));
          },
          (error) => console.log(JSON.stringify({ error: String(error) })),
        );
      `);
      assert.deepStrictEqual(result, { same: true, parts: 0 }, ended);
    });
  },
);

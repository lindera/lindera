// The batch methods of Tokenizer that return a Promise (lindera/lindera#1406):
// the texts are tokenized on a thread of libuv's pool while the event loop
// runs, and the results are those of the synchronous batch methods.
const assert = require("node:assert");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { describe, it } = require("node:test");

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

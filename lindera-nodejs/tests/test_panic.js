const assert = require("assert");
const { spawnSync } = require("child_process");
const { describe, it } = require("node:test");
const path = require("path");

const lindera = require("../index.js");

// `__panicForTest` exists only in a build with the `test-hooks` feature,
// which `make test-lindera-nodejs` enables and a release build never does.
const hasTestHooks = typeof lindera.__panicForTest === "function";

// Every exported function and method of the binding is marked
// `#[napi(catch_unwind)]`, so that a panic in Rust throws an `Error` instead
// of unwinding into napi-rs's `extern "C"` callback, which aborted Node.js
// with `fatal runtime error: failed to initiate panic, error 5, aborting`
// (#1314). A unit test in src/lib.rs checks that every function and method
// carries it; this test checks what a panic becomes at run time.
describe(
  "A panic in Rust",
  { skip: !hasTestHooks && "built without the test-hooks feature" },
  () => {
    const index = path.join(__dirname, "..", "index.js");
    const message = "lindera-nodejs test panic";

    // The panic happens in a child, so that a panic that aborts kills the
    // child alone, and the assertions below say how it ended.
    const childScript = `
      const lindera = require(${JSON.stringify(index)});
      let error;
      try {
        lindera.__panicForTest(${JSON.stringify(message)});
      } catch (e) {
        error = e;
      }
      // The binding goes on working after the panic.
      const version = lindera.version();
      const fieldCount = new lindera.Schema(["a", "b"]).fieldCount();
      console.log(JSON.stringify({
        thrown: error !== undefined,
        isError: error instanceof Error,
        code: error && error.code,
        message: error && error.message,
        version,
        fieldCount,
      }));
    `;

    it("throws an Error instead of aborting Node.js", () => {
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

      const result = JSON.parse(child.stdout);
      assert.deepStrictEqual(
        result,
        {
          thrown: true,
          isError: true,
          code: "GenericFailure",
          message,
          version: lindera.version(),
          fieldCount: 2,
        },
        ended,
      );
      // Rust's panic hook still writes the panic's location and message.
      assert.match(child.stderr, /panicked at /, ended);
      assert.ok(child.stderr.includes(message), ended);
    });
  },
);

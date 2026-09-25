const assert = require("node:assert");
const { spawnSync } = require("node:child_process");
const path = require("node:path");
const { describe, it } = require("node:test");

const { TokenizerBuilder } = require("../index.js");

const INDEX = path.join(__dirname, "..", "index.js");
const TOO_DEEP = /nested more than 128 levels deep/;

// The three builder methods that convert a JS value to JSON.
const CALLS = {
  setSpacePenalty: (builder, value) => builder.setSpacePenalty(value),
  appendCharacterFilter: (builder, value) =>
    builder.appendCharacterFilter("unicode_normalize", value),
  appendTokenFilter: (builder, value) =>
    builder.appendTokenFilter("lowercase", value),
};

// Returns `{ a: { a: ... } }` made of `levels` nested objects.
function nestedObjects(levels) {
  let value = {};
  for (let i = 1; i < levels; i++) {
    value = { a: value };
  }
  return value;
}

// Returns `[[...]]` made of `levels` nested arrays.
function nestedArrays(levels) {
  let value = [];
  for (let i = 1; i < levels; i++) {
    value = [value];
  }
  return value;
}

// Calls `call` and returns the error it throws, or null.
function thrownBy(call) {
  try {
    call();
    return null;
  } catch (error) {
    return error;
  }
}

// Runs `CALLS[method]` on a value built by `valueSource` in a separate
// process, so a stack overflow shows up as a failed assertion instead of
// killing the test runner.
function runIsolated(method, valueSource) {
  const script = `
    const { TokenizerBuilder } = require(${JSON.stringify(INDEX)});
    ${nestedObjects.toString()}
    const value = ${valueSource};
    const builder = new TokenizerBuilder();
    try {
      (${CALLS[method].toString()})(builder, value);
      process.stdout.write("returned");
    } catch (error) {
      process.stdout.write("threw: " + error.message);
    }
  `;
  return spawnSync(process.execPath, ["-e", script], {
    encoding: "utf8",
    timeout: 60_000,
  });
}

describe("argument nesting limit", () => {
  for (const method of Object.keys(CALLS)) {
    describe(method, () => {
      it("accepts 128 levels", () => {
        for (const value of [nestedObjects(128), nestedArrays(128)]) {
          const error = thrownBy(() =>
            CALLS[method](new TokenizerBuilder(), value),
          );
          // setSpacePenalty rejects the shape, but not for being too deep.
          if (error !== null) {
            assert.doesNotMatch(error.message, TOO_DEEP);
          }
        }
      });

      it("rejects 129 levels with a catchable error", () => {
        for (const value of [nestedObjects(129), nestedArrays(129)]) {
          assert.throws(
            () => CALLS[method](new TokenizerBuilder(), value),
            TOO_DEEP,
          );
        }
      });

      it("keeps the builder usable after rejecting an argument", () => {
        const builder = new TokenizerBuilder();
        assert.throws(() => CALLS[method](builder, nestedObjects(129)), TOO_DEEP);
        assert.doesNotThrow(() => CALLS[method](builder, null));
      });

      for (const [name, valueSource] of [
        ["a value that contains itself", "(() => { const o = { rules: [] }; o.self = o; return o; })()"],
        ["10,000 levels", "nestedObjects(10000)"],
      ]) {
        it(`rejects ${name} without crashing the process`, () => {
          const result = runIsolated(method, valueSource);
          assert.strictEqual(result.signal, null, result.stderr);
          assert.strictEqual(result.status, 0, result.stderr);
          assert.match(result.stdout, /^threw: /);
          assert.match(result.stdout, TOO_DEEP);
        });
      }
    });
  }

  // The limit is enforced by our own walk instead of napi-rs's conversion;
  // these pin down that valid and invalid values still convert as before.
  describe("conversion parity", () => {
    it("skips properties whose value is undefined", () => {
      assert.doesNotThrow(() =>
        new TokenizerBuilder().appendTokenFilter("lowercase", {
          a: undefined,
          b: [1, "x", null, true, 10n],
        }),
      );
    });

    it("rejects an undefined array element as before", () => {
      assert.throws(
        () => new TokenizerBuilder().appendTokenFilter("lowercase", { a: [undefined] }),
        /undefined cannot be represented as a serde_json::Value/,
      );
    });

    it("rejects a function as before", () => {
      assert.throws(
        () => new TokenizerBuilder().appendTokenFilter("lowercase", { a: () => 1 }),
        /JS functions cannot be represented as a serde_json::Value/,
      );
    });
  });
});

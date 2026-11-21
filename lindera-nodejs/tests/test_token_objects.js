// The token objects and N-best results of every method that returns them
// (lindera/lindera#1409). The binding makes them with a conversion of its
// own; they must be the objects that the conversion napi-rs derives from
// `Token` and `NbestResult` makes, which the test hooks still give: the same
// values, fields in the same order, the same property attributes, plain
// objects on Object.prototype.
const assert = require("node:assert");
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

// The hooks exist only in a build with the `test-hooks` feature, which
// `make test-lindera-nodejs` enables and a release build never does.
const hasTestHooks =
  typeof lindera.__tokenizeBatchDerivedForTest === "function";

const bocchan = path.join(__dirname, "..", "..", "resources", "bocchan.txt");

function tokenizer() {
  return new lindera.Tokenizer(
    lindera.loadDictionary("embedded://ipadic"),
    lindera.Mode.normal(),
  );
}

// Every line of bocchan.txt.
function allLines() {
  return fs.readFileSync(bocchan, "utf8").split("\n");
}

// 40 lines, an empty text, and the 40 lines as one text.
function partTexts() {
  const lines = allLines().slice(0, 40);
  return lines.concat(["", lines.join("")]);
}

// The first place where `actual` and `expected` differ, or null: their
// values and types, the order of the fields of every object, its prototype,
// and the attributes of its own properties.
function difference(actual, expected, at) {
  if (Array.isArray(expected)) {
    if (!Array.isArray(actual)) {
      return `${at}: not an array`;
    }
    if (actual.length !== expected.length) {
      return `${at}: ${actual.length} elements instead of ${expected.length}`;
    }
    for (let i = 0; i < expected.length; i += 1) {
      const found = difference(actual[i], expected[i], `${at}[${i}]`);
      if (found) {
        return found;
      }
    }
    return null;
  }
  if (expected !== null && typeof expected === "object") {
    if (actual === null || typeof actual !== "object") {
      return `${at}: not an object`;
    }
    if (Object.getPrototypeOf(actual) !== Object.prototype) {
      return `${at}: not on Object.prototype`;
    }
    const got = Object.getOwnPropertyDescriptors(actual);
    const want = Object.getOwnPropertyDescriptors(expected);
    const keys = Object.keys(want).join(",");
    if (Object.keys(got).join(",") !== keys) {
      return `${at}: fields ${Object.keys(got).join(",")} instead of ${keys}`;
    }
    for (const key of Object.keys(want)) {
      for (const attribute of ["writable", "enumerable", "configurable"]) {
        if (got[key][attribute] !== want[key][attribute]) {
          return `${at}.${key}: ${attribute} ${got[key][attribute]} instead of ${want[key][attribute]}`;
        }
      }
      const found = difference(actual[key], expected[key], `${at}.${key}`);
      if (found) {
        return found;
      }
    }
    return null;
  }
  if (typeof actual !== typeof expected || actual !== expected) {
    return `${at}: ${JSON.stringify(actual)} (${typeof actual}) instead of ${JSON.stringify(expected)} (${typeof expected})`;
  }
  return null;
}

function assertSameObjects(actual, expected, label) {
  const found = difference(actual, expected, label);
  if (found) {
    assert.fail(found);
  }
}

// Runs `call` with a hook set to `value`, then puts its default back.
function withHook(hook, value, call) {
  lindera[hook](value);
  try {
    return call();
  } finally {
    lindera[hook](null);
  }
}

describe(
  "Token objects",
  {
    skip:
      (!hasEmbeddedIpadic && "embedded://ipadic not available") ||
      (!hasTestHooks && "built without the test-hooks feature"),
  },
  () => {
    // The objects of each method against those of the derived conversion.
    function checkAll(t, texts, label) {
      const derived = lindera.__tokenizeBatchDerivedForTest(t, texts);
      assertSameObjects(
        t.tokenizeBatch(texts),
        derived,
        `${label} tokenizeBatch`,
      );
      for (let i = 0; i < texts.length; i += 1) {
        assertSameObjects(
          t.tokenize(texts[i]),
          derived[i],
          `${label} tokenize(${i})`,
        );
      }
      const some = texts.slice(0, 12).concat(texts.slice(-2));
      const nbest = some.map((text) =>
        lindera.__tokenizeNbestDerivedForTest(t, text, 3),
      );
      assertSameObjects(
        t.tokenizeNbestBatch(some, 3),
        nbest,
        `${label} tokenizeNbestBatch`,
      );
      for (let i = 0; i < some.length; i += 1) {
        assertSameObjects(
          t.tokenizeNbest(some[i], 3),
          nbest[i],
          `${label} tokenizeNbest(${i})`,
        );
      }
    }

    it("makes the objects of the derived conversion", () => {
      const t = tokenizer();
      checkAll(t, allLines(), "bocchan.txt");
      checkAll(t, partTexts(), "40 lines");
    });

    for (const tokens of [1, 7, 50]) {
      it(`makes the same objects in handle scopes of ${tokens} tokens`, () => {
        const t = tokenizer();
        withHook("__setTokenScopeForTest", tokens, () =>
          checkAll(t, partTexts(), `scopes of ${tokens}`),
        );
      });
    }

    for (const mode of ["thread", "inline"]) {
      it(`makes the same objects with the views dropped ${mode === "thread" ? "on their thread" : "on the spot"}`, () => {
        const t = tokenizer();
        const texts = partTexts();
        const derived = lindera.__tokenizeBatchDerivedForTest(t, texts);
        withHook("__setDropThreadForTest", mode, () => {
          for (let round = 0; round < 3; round += 1) {
            assertSameObjects(
              t.tokenizeBatch(texts),
              derived,
              `${mode} round ${round}`,
            );
            assertSameObjects(
              t.tokenize(texts[0]),
              derived[0],
              `${mode} round ${round} tokenize`,
            );
          }
        });
      });
    }

    it("makes no token for an empty text or batch", () => {
      const t = tokenizer();
      assertSameObjects(t.tokenize(""), [], "tokenize");
      assertSameObjects(t.tokenizeBatch([]), [], "tokenizeBatch");
      assertSameObjects(
        t.tokenizeBatch(["", ""]),
        [[], []],
        "tokenizeBatch of empty texts",
      );
      assertSameObjects(t.tokenizeNbestBatch([], 2), [], "tokenizeNbestBatch");
    });
  },
);

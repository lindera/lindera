// The batch methods of Tokenizer (lindera/lindera#1402): every text is
// tokenized as a call of its own, on several threads, and the results come
// in the order of the texts, the same as the single calls'.
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

// The lines of resources/bocchan.txt and an empty text: enough input for a
// batch on several threads.
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

describe(
  "Tokenizer batch methods",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    for (const maxThreads of [undefined, null, 1, 2, 8]) {
      it(`tokenizeBatch gives tokenize's tokens (maxThreads ${maxThreads})`, () => {
        const t = tokenizer();
        const all = texts();
        const expected = all.map((text) => t.tokenize(text));
        assert.deepStrictEqual(t.tokenizeBatch(all, maxThreads), expected);
      });
    }

    it("tokenizeSurfacesBatch gives tokenizeSurfaces's surfaces", () => {
      const t = tokenizer();
      const all = texts();
      const expected = all.map((text) => t.tokenizeSurfaces(text));
      assert.deepStrictEqual(t.tokenizeSurfacesBatch(all, 4), expected);
      assert.deepStrictEqual(t.tokenizeSurfacesBatch(all), expected);
    });

    it("tokenizeNbestBatch gives tokenizeNbest's results", () => {
      const t = tokenizer();
      const some = texts().slice(0, 60);
      const expected = some.map((text) => t.tokenizeNbest(text, 3, true, 5000));
      assert.deepStrictEqual(
        t.tokenizeNbestBatch(some, 3, true, 5000, 4),
        expected,
      );
      assert.deepStrictEqual(
        t.tokenizeNbestBatch(some, 3, true, 5000),
        expected,
      );
    });

    it("tokenizeBatch takes an empty array", () => {
      assert.deepStrictEqual(tokenizer().tokenizeBatch([]), []);
    });

    it("tokenizeBatch rejects a string instead of an array", () => {
      assert.throws(() => tokenizer().tokenizeBatch("東京"));
    });
  },
);

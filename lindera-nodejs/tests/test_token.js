const assert = require("assert");
const { describe, it, before } = require("node:test");

const lindera = require("../index.js");

// Skip all tests if embedded IPADIC is not available
const hasEmbeddedIpadic = (() => {
  try {
    lindera.loadDictionary("embedded://ipadic");
    return true;
  } catch {
    return false;
  }
})();

describe(
  "Token",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    let tokenizer;

    before(() => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      tokenizer = new lindera.Tokenizer(dictionary, "normal");
    });

    it("should return Token objects from tokenize", () => {
      const tokens = tokenizer.tokenize("関西国際空港");

      assert.ok(Array.isArray(tokens));
      assert.ok(tokens.length > 0);

      const token = tokens[0];

      // Check surface attribute
      assert.strictEqual(typeof token.surface, "string");
      assert.ok(token.surface.startsWith("関西"));

      // Check other attributes
      assert.strictEqual(typeof token.byteStart, "number");
      assert.strictEqual(typeof token.byteEnd, "number");
      assert.strictEqual(typeof token.position, "number");
      assert.strictEqual(typeof token.wordId, "number");
      assert.strictEqual(typeof token.isUnknown, "boolean");

      // Check details
      assert.ok(Array.isArray(token.details));
      assert.ok(token.details.length > 0);
    });

    it("should expose details by index", () => {
      const tokens = tokenizer.tokenize("東京");
      const token = tokens[0];

      assert.ok(token.details.length > 0);
      assert.strictEqual(typeof token.details[0], "string");

      // Out of bounds reads yield undefined, as for any JS array.
      assert.strictEqual(token.details[9999], undefined);
    });

    // Tokens are plain objects rather than class instances so that their
    // memory is owned by the JS heap: napi defers class finalizers to the
    // event loop, which accumulates native memory in synchronous loops that
    // never yield (#922, #930).
    it("should return plain objects, not class instances", () => {
      const tokens = tokenizer.tokenize("テスト");

      assert.ok(tokens.length > 0);
      const token = tokens[0];

      // Plain objects: data properties, not prototype getters.
      assert.ok(Object.getOwnPropertyDescriptor(token, "surface"));
      assert.strictEqual(Object.getPrototypeOf(token), Object.prototype);
    });

    it("should survive JSON and structuredClone round-trips", () => {
      const token = tokenizer.tokenize("テスト")[0];

      const parsed = JSON.parse(JSON.stringify(token));
      assert.strictEqual(parsed.surface, token.surface);
      assert.strictEqual(parsed.byteStart, token.byteStart);
      assert.strictEqual(parsed.isUnknown, token.isUnknown);
      assert.deepStrictEqual(parsed.details, token.details);

      const cloned = structuredClone(token);
      assert.deepStrictEqual(cloned, token);
    });
  },
);

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

    it("should support getDetail method", () => {
      const tokens = tokenizer.tokenize("東京");
      const token = tokens[0];

      const firstDetail = token.getDetail(0);
      assert.ok(firstDetail !== null);
      assert.strictEqual(firstDetail, token.details[0]);

      // Out of bounds
      assert.strictEqual(token.getDetail(9999), null);
    });
  },
);

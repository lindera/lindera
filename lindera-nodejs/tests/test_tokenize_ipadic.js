const assert = require("assert");
const { describe, it } = require("node:test");

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
  "Tokenize with IPADIC",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    it("should tokenize Japanese text correctly", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const tokenizer = new lindera.Tokenizer(dictionary, "normal");

      const tokens = tokenizer.tokenize("すもももももももものうち");

      assert.strictEqual(tokens[0].surface, "すもも");
      assert.strictEqual(tokens[1].surface, "も");
      assert.strictEqual(tokens[2].surface, "もも");
      assert.strictEqual(tokens[3].surface, "も");
      assert.strictEqual(tokens[4].surface, "もも");
      assert.strictEqual(tokens[5].surface, "の");
      assert.strictEqual(tokens[6].surface, "うち");

      assert.strictEqual(tokens.length, 7);
    });

    it("should tokenize using TokenizerBuilder", () => {
      const builder = new lindera.TokenizerBuilder();
      builder.setMode("normal");
      builder.setDictionary("embedded://ipadic");
      const tokenizer = builder.build();

      const tokens = tokenizer.tokenize("東京タワー");
      assert.ok(tokens.length > 0);
      assert.strictEqual(tokens[0].surface, "東京");
    });

    it("should support N-best tokenization", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const tokenizer = new lindera.Tokenizer(dictionary, "normal");

      const results = tokenizer.tokenizeNbest("すもももももももものうち", 3);

      assert.ok(Array.isArray(results));
      assert.ok(results.length > 0);
      assert.ok(results.length <= 3);

      for (const result of results) {
        assert.ok(Array.isArray(result.tokens));
        assert.strictEqual(typeof result.cost, "number");
        assert.ok(result.tokens.length > 0);
      }
    });
  },
);

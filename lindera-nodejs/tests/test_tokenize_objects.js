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
  "tokenizeObjects",
  { skip: !hasEmbeddedIpadic && "embedded://ipadic not available" },
  () => {
    it("should return plain objects matching tokenize output", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const tokenizer = new lindera.Tokenizer(dictionary, "normal");

      const text = "すもももももももものうち";
      const classTokens = tokenizer.tokenize(text);
      const objectTokens = tokenizer.tokenizeObjects(text);

      assert.strictEqual(objectTokens.length, classTokens.length);
      for (let i = 0; i < classTokens.length; i++) {
        assert.strictEqual(objectTokens[i].surface, classTokens[i].surface);
        assert.strictEqual(objectTokens[i].byteStart, classTokens[i].byteStart);
        assert.strictEqual(objectTokens[i].byteEnd, classTokens[i].byteEnd);
        assert.strictEqual(objectTokens[i].position, classTokens[i].position);
        assert.strictEqual(objectTokens[i].wordId, classTokens[i].wordId);
        assert.strictEqual(objectTokens[i].isUnknown, classTokens[i].isUnknown);
        assert.deepStrictEqual(objectTokens[i].details, classTokens[i].details);
      }
    });

    it("should return plain objects, not class instances", () => {
      const dictionary = lindera.loadDictionary("embedded://ipadic");
      const tokenizer = new lindera.Tokenizer(dictionary, "normal");

      const tokens = tokenizer.tokenizeObjects("テスト");

      assert.ok(tokens.length > 0);
      // Plain objects: data properties, not prototype getters.
      const token = tokens[0];
      assert.ok(Object.getOwnPropertyDescriptor(token, "surface"));
      assert.strictEqual(Object.getPrototypeOf(token), Object.prototype);
      // Round-trips through JSON with all fields intact.
      const parsed = JSON.parse(JSON.stringify(token));
      assert.strictEqual(parsed.surface, token.surface);
      assert.deepStrictEqual(parsed.details, token.details);
    });
  }
);

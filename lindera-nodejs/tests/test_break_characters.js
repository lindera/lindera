const assert = require("node:assert");
const { describe, it } = require("node:test");

const { TokenizerBuilder, loadDictionary } = require("../index.js");

// Matches the guard in test_tokenize_ipadic.js: the embedded dictionary is behind the
// `embed-ipadic` feature, which is not on by default.
const hasEmbeddedIpadic = (() => {
  try {
    loadDictionary("embedded://ipadic");
    return true;
  } catch {
    return false;
  }
})();

// "レノン・ジョン" is not in IPADIC. With "・" as a break character (the default),
// the unknown word stops at it; with none, the katakana run, "・" included, is one
// unknown word, as in MeCab (lindera/lindera#1353).
const TEXT = "レノン・ジョン";
const SPLIT = ["レノン", "・", "ジョン"];
const WHOLE = ["レノン・ジョン"];

const surfaces = (builder) =>
  builder
    .build()
    .tokenize(TEXT)
    .map((t) => t.surface);

const ipadicBuilder = (...values) => {
  const builder = new TokenizerBuilder().setDictionary("embedded://ipadic");
  for (const value of values) {
    builder.setBreakCharacters(value);
  }
  return builder;
};

describe("TokenizerBuilder.setBreakCharacters", () => {
  it("accepts strings, null and undefined, and returns the builder", () => {
    for (const value of ["・", "", null, undefined]) {
      const builder = new TokenizerBuilder();
      assert.strictEqual(builder.setBreakCharacters(value), builder);
    }
  });

  it("rejects values that are not strings", () => {
    for (const value of [0, true, ["・"]]) {
      assert.throws(
        () => new TokenizerBuilder().setBreakCharacters(value),
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it(
    "sets the break characters with IPADIC",
    {
      skip: !hasEmbeddedIpadic && "embedded://ipadic not available",
    },
    () => {
      const cases = [
        [[], SPLIT],
        [["・"], SPLIT],
        [[""], WHOLE],
        // Another set replaces the default.
        [["ー"], WHOLE],
        // null and undefined go back to the default, "・".
        [["", null], SPLIT],
        [["", undefined], SPLIT],
        // The last call wins.
        [["・", ""], WHOLE],
      ];
      for (const [values, expected] of cases) {
        assert.deepStrictEqual(
          surfaces(ipadicBuilder(...values)),
          expected,
          JSON.stringify(values),
        );
      }
    },
  );
});

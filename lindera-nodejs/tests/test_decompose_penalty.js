// The penalty of decompose mode: TokenizerBuilder.setMode and the Tokenizer
// constructor take a Penalty object (lindera/lindera#1215).
const assert = require("node:assert");
const { describe, it } = require("node:test");

const { Tokenizer, TokenizerBuilder, loadDictionary } = require("../index.js");

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
const skip = !hasEmbeddedIpadic && "embedded://ipadic not available";

const TEXT = "関西国際空港限定トートバッグ";
const DEFAULT_SURFACES = ["関西", "国際", "空港", "限定", "トートバッグ"];
// Without length penalties, decompose mode keeps the compound whole, as
// normal mode does.
const NO_LENGTH_PENALTY = { kanjiPenaltyLengthPenalty: 0, otherPenaltyLengthPenalty: 0 };
const NO_LENGTH_PENALTY_SURFACES = ["関西国際空港", "限定", "トートバッグ"];

// Returns the surfaces of TEXT for a tokenizer built with setMode(...args).
const builderSurfaces = (...args) => {
  const builder = new TokenizerBuilder();
  builder.setDictionary("embedded://ipadic");
  builder.setMode(...args);
  return builder.build().tokenize(TEXT).map((token) => token.surface);
};

// Returns the surfaces of TEXT for a Tokenizer built directly with these
// arguments after the dictionary.
const constructorSurfaces = (...args) =>
  new Tokenizer(loadDictionary("embedded://ipadic"), ...args)
    .tokenize(TEXT)
    .map((token) => token.surface);

describe("TokenizerBuilder.setMode penalty", () => {
  it("reaches the segmenter and keeps the default when omitted", { skip }, () => {
    assert.deepStrictEqual(builderSurfaces("decompose"), DEFAULT_SURFACES, "omitted");
    assert.deepStrictEqual(builderSurfaces("decompose", undefined), DEFAULT_SURFACES, "undefined");
    assert.deepStrictEqual(builderSurfaces("decompose", null), DEFAULT_SURFACES, "null");
    assert.deepStrictEqual(builderSurfaces("decompose", {}), DEFAULT_SURFACES, "{}");
    assert.deepStrictEqual(
      builderSurfaces("decompose", NO_LENGTH_PENALTY),
      NO_LENGTH_PENALTY_SURFACES,
      "no length penalty",
    );
  });

  it("keeps the default of a field that is left out", { skip }, () => {
    // With only the kanji penalty 0, the other penalty does not matter for
    // this text, whose long words are all kanji.
    assert.deepStrictEqual(
      builderSurfaces("decompose", { kanjiPenaltyLengthPenalty: 0 }),
      NO_LENGTH_PENALTY_SURFACES,
    );
  });

  it("returns the builder", () => {
    const builder = new TokenizerBuilder();
    assert.strictEqual(builder.setMode("decompose", NO_LENGTH_PENALTY), builder);
  });

  it("rejects a penalty with normal mode", () => {
    assert.throws(() => new TokenizerBuilder().setMode("normal", NO_LENGTH_PENALTY), {
      message: /decompose/,
    });
  });

  it("rejects a penalty that is not an object", () => {
    for (const value of [0, "penalty", true]) {
      assert.throws(
        () => new TokenizerBuilder().setMode("decompose", value),
        { code: "InvalidArg", message: /Expect value to be Object/ },
        `value ${JSON.stringify(value)}`,
      );
    }
  });

  it("rejects a field that is not a number", () => {
    for (const value of [{ kanjiPenaltyLengthPenalty: "0" }, { kanjiPenaltyLengthPenalty: null }]) {
      assert.throws(
        () => new TokenizerBuilder().setMode("decompose", value),
        { code: "NumberExpected", message: /kanjiPenaltyLengthPenalty/ },
        `value ${JSON.stringify(value)}`,
      );
    }
  });
});

describe("Tokenizer constructor penalty", () => {
  it("reaches the segmenter and keeps the default when omitted", { skip }, () => {
    assert.deepStrictEqual(constructorSurfaces("decompose"), DEFAULT_SURFACES, "omitted");
    assert.deepStrictEqual(
      constructorSurfaces("decompose", null, null, null),
      DEFAULT_SURFACES,
      "null",
    );
    assert.deepStrictEqual(
      constructorSurfaces("decompose", null, undefined, NO_LENGTH_PENALTY),
      NO_LENGTH_PENALTY_SURFACES,
      "no length penalty",
    );
  });

  it("rejects a penalty with normal mode", { skip }, () => {
    assert.throws(() => constructorSurfaces("normal", null, null, NO_LENGTH_PENALTY), {
      message: /decompose/,
    });
  });

  it("rejects a penalty that is not an object", { skip }, () => {
    assert.throws(() => constructorSurfaces("decompose", null, null, 0), {
      code: "InvalidArg",
      message: /Expect value to be Object/,
    });
  });
});
